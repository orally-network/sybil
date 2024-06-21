use std::collections::HashMap;
use std::pin::Pin;

use candid::CandidType;
use derivative::Derivative;
use futures::Future;
use ic_cdk::api::management_canister::ecdsa::{EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgument};
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpResponse,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use hex::FromHexError;
use ic_web3_rs::signing::keccak256;
use serde_json::Error as SerdeError;
use thiserror::Error;

use crate::utils::signature::{get_eth_v, sign};
use crate::{
    clone_with_state, log,
    utils::{
        address::{self, AddressError},
        canister::{self, CanisterError},
        nat,
        signature::SignatureError,
        time,
    },
};
use crate::{HTTP_CACHE, SIGNATURES_CACHE, UNIVERSAL_CACHE};

use super::feed_types::rate_data::AssetDataResult;
use super::{Seconds, Timestamp};

const HTTP_WAITING_DELAY_SECS: u64 = 3;
const HTTP_WATTING_TIMEOUT_SECS: u64 = 24;
const HTTP_OUTCALL_REQUEST_CYCLES: u128 = 400_000_000;
const HTTP_OUTCALL_PAYLOAD_CYCLES: u128 = 100_000;
const MAX_RESPONSE_BYTES: u128 = 2 * 1024 * 1024; // 2 MB
const CACHE_TTL_SEC: u64 = 10 * 60; // 10 minutes

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct RateCache(HashMap<String, RateCacheEntry>);

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
struct RateCacheEntry {
    expired_at: u64,
    data: AssetDataResult,
}

impl RateCache {
    pub fn add_entry(&mut self, key: String, data: AssetDataResult, expiration: Seconds) {
        let entry = RateCacheEntry {
            expired_at: time::in_seconds() + expiration,
            data,
        };

        self.0.insert(key, entry);
    }

    pub fn get_entry(&mut self, key: &str) -> Option<AssetDataResult> {
        let entry = self.0.get(key);

        if let Some(entry) = entry {
            if entry.expired_at < time::in_seconds() {
                self.0.remove(key);
                return None;
            }

            return Some(entry.data.clone());
        }

        None
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub struct HttpCache {
    entries: HashMap<String, HttpCacheEntry>,
    #[derivative(Default(value = "300"))]
    capacity: usize,
    stats: HttpCacheStats,
}

#[derive(Default, Clone, CandidType, Serialize, Deserialize, Debug)]
struct HttpCacheEntry {
    cached_at: Timestamp,
    // expiration frequency
    expr_freq: Seconds,
    response: Option<HttpResponse>,
}

impl HttpCacheEntry {
    fn new(response: HttpResponse, expr_freq: Seconds) -> Self {
        Self {
            cached_at: time::in_seconds(),
            expr_freq,
            response: Some(response),
        }
    }

    fn is_expired(&self) -> bool {
        self.cached_at + self.expr_freq < time::in_seconds()
    }
}

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct HttpCacheStats {
    hits: usize,
    misses: usize,
    cache_size: usize,
    total_requests: usize,
}

#[derive(Error, Debug)]
pub enum HttpCacheError {
    #[error("HTTP outcall error with message: {0}")]
    HttpOutcallError(String),
    #[error("Got error from server: {0}")]
    ServerError(String),
    #[error("Invalid response body json: {0}")]
    InvalidResponseBodyJson(#[from] SerdeError),
    #[error("Invalid response body resolver: {0}")]
    InvalidResponseBodyResolver(String),
}

impl HttpCache {
    pub async fn request_with_access(
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<(HttpResponse, Seconds), HttpCacheError> {
        let mut cache = HTTP_CACHE.with(|c| c.borrow().clone());
        let response = cache.request(request, expr_freq).await;
        HTTP_CACHE.with(|c| c.replace(cache));
        response
    }

    pub async fn request_with_access_w3(
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<(HttpResponse, Seconds), HttpCacheError> {
        let mut cache = HTTP_CACHE.with(|c| c.borrow().clone());
        let response = cache.request(request, expr_freq).await;
        HTTP_CACHE.with(|c| c.replace(cache));
        response
    }

    pub async fn request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<(HttpResponse, Seconds), HttpCacheError> {
        log!("[HTTP CACHE] got request");
        self.stats.total_requests += 1;

        let Some(entry) = self.entries.get(&request.url) else {
            log!("[HTTP CACHE] record not found in cache");
            return self.force_request(request, expr_freq).await;
        };

        if let Some(response) = &entry.response {
            log!("[HTTP CACHE] response found in cache");
            if entry.is_expired() {
                log!("[HTTP CACHE] response expired");
                return self.force_request(request, expr_freq).await;
            }
            return Ok((response.clone(), entry.cached_at));
        }

        log!("[HTTP CACHE] response not found in cache");
        // waiting for the pending request to finish
        for _ in 0..(HTTP_WATTING_TIMEOUT_SECS / HTTP_WAITING_DELAY_SECS) {
            time::wait(HTTP_WAITING_DELAY_SECS).await;

            if let Some(response) = &entry.response {
                return Ok((response.clone(), entry.cached_at));
            }
        }

        self.force_request(request, expr_freq).await
    }

    pub async fn force_request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<(HttpResponse, Seconds), HttpCacheError> {
        let mut cycles =
            HTTP_OUTCALL_REQUEST_CYCLES + (MAX_RESPONSE_BYTES * HTTP_OUTCALL_PAYLOAD_CYCLES);
        if let Some(body) = &request.body {
            cycles += body.len() as u128 * HTTP_OUTCALL_PAYLOAD_CYCLES;
        }

        self.entries
            .insert(request.url.clone(), HttpCacheEntry::default());

        let response = http_request(request.clone(), cycles)
            .await
            .map_err(|(_, msg)| {
                self.stats.misses += 1;
                HttpCacheError::HttpOutcallError(msg)
            })?
            .0;

        if nat::to_u64(&response.status) >= 400 {
            let msg =
                String::from_utf8(response.body).unwrap_or_else(|_| "unknown error".to_string());

            self.stats.misses += 1;

            return Err(HttpCacheError::ServerError(msg));
        }

        let entry = self.entries.get_mut(&request.url);
        if let Some(entry) = entry {
            entry.response = Some(response.clone());
            entry.cached_at = time::in_seconds();
            entry.expr_freq = expr_freq;
        } else {
            self.entries.insert(
                request.url.clone(),
                HttpCacheEntry::new(response.clone(), expr_freq),
            );
        }

        self.stats.hits += 1;

        let entry = self.entries.get(&request.url).expect("entry not found");

        Ok((response, entry.cached_at))
    }

    pub fn clean(&mut self) {
        if self.entries.len() <= self.capacity {
            return;
        }

        self.entries.retain(|_, entry| !entry.is_expired());

        let mut entries = Vec::from_iter(self.entries.iter());

        entries.sort_by(|(_, first), (_, second)| first.cached_at.cmp(&second.cached_at));

        let keys = entries
            .into_iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<String>>();

        let (keys_to_remove, _) = keys.split_at(keys.len().saturating_sub(self.capacity));

        self.entries.retain(|key, _| !keys_to_remove.contains(key));
    }

    pub fn stats(&self) -> HttpCacheStats {
        let mut stats = self.stats.clone();
        stats.cache_size = self.entries.len();
        stats
    }
}

#[derive(Error, Debug)]
pub enum SignaturesCacheError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(#[from] FromHexError),
    #[error("Unable to sign with ecdsa: {0}")]
    SignWithECDSAError(String),
    #[error("Canister error: {0}")]
    CanisterError(#[from] CanisterError),
    #[error("Address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("Signature error: {0}")]
    SignatureError(#[from] SignatureError),
}

#[derive(Debug, Clone, CandidType, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SignaturesCache {
    signatures: HashMap<String, String>,
    #[derivative(Default(value = "300"))]
    limit: usize,
}

impl SignaturesCache {
    pub async fn eth_sign_with_access(data: &[u8]) -> Result<Vec<u8>, SignaturesCacheError> {
        let mut cache = SIGNATURES_CACHE.with(|c| c.borrow().clone());
        let signature = cache.eth_sign(data).await;
        SIGNATURES_CACHE.with(|c| c.replace(cache));
        signature
    }

    pub async fn eth_sign(&mut self, data: &[u8]) -> Result<Vec<u8>, SignaturesCacheError> {
        let sign_data = keccak256(data).to_vec();
        if let Some(signature) = self.signatures.get(&hex::encode(&sign_data)) {
            log!("[SIGNATURE CACHE] signature found in cache");
            return Ok(hex::decode(signature)?);
        }

        let key_name = clone_with_state!(key_name);
        let call_args = SignWithEcdsaArgument {
            message_hash: sign_data.clone(),
            derivation_path: vec![ic_cdk::id().as_slice().to_vec()],
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_name,
            },
        };

        let mut signature = sign(call_args)
            .await
            .map_err(|(_, msg)| SignaturesCacheError::SignWithECDSAError(msg))?
            .0
            .signature;

        let pub_key = canister::eth_address().await?;

        signature.push(get_eth_v(
            &signature,
            &sign_data,
            &address::to_h160(&pub_key)?,
        )?);

        self.signatures
            .insert(hex::encode(&sign_data), hex::encode(&signature));

        log!("[SIGNATURE CACHE] signature was not found in cache");
        Ok(signature)
    }

    pub fn clean(&mut self) {
        if self.signatures.len() <= self.limit {
            return;
        }

        let mut entries = Vec::from_iter(self.signatures.iter());

        entries.sort_by(|(_, first), (_, second)| first.cmp(second));

        let keys = entries
            .into_iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<String>>();

        let (keys_to_remove, _) = keys.split_at(keys.len().saturating_sub(self.limit));

        self.signatures
            .retain(|key, _| !keys_to_remove.contains(key));
    }
}

#[derive(Error, Debug, CandidType, Deserialize)]
pub enum CacheError {
    #[error("Cannot deserialize data: {0}")]
    DeserializationError(String),
    #[error("Cannot serialize data: {0}")]
    SerializationError(String),
    #[error("Cannot get data: {0}")]
    GetDataError(String),
}

#[derive(Debug, Clone, CandidType)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub expires_at: u64,
}

/// Cache for storing data.
/// Stores data for separate cache_ttl time and separate func_signature.
/// Cache: cache_ttl -> func_signature -> cache_entry
/// User could specify cache_ttl, then the data will be stored for this ttl separately.
#[derive(Debug, Clone, Default, CandidType)]
pub struct Cache(HashMap<u64, HashMap<String, CacheEntry>>);

/// CacheBuilder is a builder for Cache.
/// If data is not found or expired, then call `func`` to get data and store it in cache with `cache_ttl` time to live.
/// If `cache_ttl` is not specified, then use default value.
/// If data has been found in cache, then update it with `on_found` function if one is given and return data.
pub struct CacheBuilder<T, E, Fut>
where
    T: Serialize + DeserializeOwned,
    E: std::error::Error + std::convert::From<CacheError>,
    Fut: Future<Output = Result<T, E>>,
{
    cache_ttl: Option<u64>,
    // callback function that will be called if data is found in cache
    on_found: Option<Box<dyn FnOnce(&mut T)>>,
    // callback function that will be called after saving data to cache
    on_save: Option<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    func_signature: String,
    func: Fut,
}

impl<T, E, Fut> CacheBuilder<T, E, Fut>
where
    T: Serialize + DeserializeOwned,
    E: std::error::Error + std::convert::From<CacheError>,
    Fut: Future<Output = Result<T, E>>,
{
    fn new(func_signature: String, func: Fut) -> Self {
        Self {
            cache_ttl: None,
            on_found: None,
            on_save: None,
            func_signature,
            func,
        }
    }

    pub fn with_cache_ttl(&mut self, cache_ttl: u64) {
        self.cache_ttl = Some(cache_ttl);
    }

    pub fn with_on_found<F>(&mut self, on_found: F)
    where
        F: FnOnce(&mut T) + 'static,
    {
        self.on_found = Some(Box::new(on_found));
    }

    pub fn with_on_save<F>(&mut self, on_save: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.on_save = Some(Box::pin(on_save));
    }

    pub async fn evaluate(self) -> Result<T, E> {
        let cache_ttl = self.cache_ttl.unwrap_or(CACHE_TTL_SEC);
        let cache = UNIVERSAL_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            let cache_by_ttl = cache.0.entry(cache_ttl).or_default();

            let entry = cache_by_ttl.get(&self.func_signature);

            if let Some(entry) = entry {
                if entry.expires_at < time::in_seconds() {
                    log!("Cache entry expired");
                    cache_by_ttl.remove(&self.func_signature);
                } else {
                    log!("Cache entry found");
                    return Some(
                        serde_cbor::from_slice(&entry.data)
                            .map_err(|err| CacheError::DeserializationError(format!("{}", err))),
                    );
                }
            }

            None
        });

        if let Some(data) = cache {
            let mut found_data = data?;
            if let Some(on_found) = self.on_found {
                on_found(&mut found_data);
            }

            return Ok(found_data);
        }

        let data = self
            .func
            .await
            .map_err(|err| CacheError::GetDataError(format!("{}", err)))?;

        let data_serialized = serde_cbor::to_vec(&data)
            .map_err(|err| CacheError::SerializationError(format!("{}", err)))?;

        UNIVERSAL_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            let cache_by_ttl = cache.0.entry(cache_ttl).or_default();

            cache_by_ttl.insert(
                self.func_signature,
                CacheEntry {
                    data: data_serialized,
                    expires_at: time::in_seconds() + cache_ttl,
                },
            );
        });

        if let Some(on_save) = self.on_save {
            on_save.await;
        }

        Ok(data)
    }
}

impl Cache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn with<T, E, Fut>(key: String, func: Fut) -> CacheBuilder<T, E, Fut>
    where
        T: Serialize + DeserializeOwned,
        E: std::error::Error + std::convert::From<CacheError>,
        Fut: Future<Output = Result<T, E>>,
    {
        CacheBuilder::new(key, func)
    }

    pub fn get_cache<T>(key: String, cache_ttl: Option<u64>) -> Option<T>
    where
        T: Serialize + DeserializeOwned,
    {
        let cache_ttl = cache_ttl.unwrap_or(CACHE_TTL_SEC);
        UNIVERSAL_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            let cache_by_ttl = cache.0.entry(cache_ttl).or_default();

            let entry = cache_by_ttl.get(&key);

            if let Some(entry) = entry {
                if entry.expires_at < time::in_seconds() {
                    cache_by_ttl.remove(&key);
                } else {
                    return Some(serde_cbor::from_slice(&entry.data).unwrap());
                }
            }

            None
        })
    }

    pub fn save_cache<T>(key: String, data: T, cache_ttl: Option<u64>)
    where
        T: Serialize,
    {
        let cache_ttl = cache_ttl.unwrap_or(CACHE_TTL_SEC);
        let data_serialized = serde_cbor::to_vec(&data).unwrap();

        UNIVERSAL_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            let cache_by_ttl = cache.0.entry(cache_ttl).or_default();

            cache_by_ttl.insert(
                key,
                CacheEntry {
                    data: data_serialized,
                    expires_at: time::in_seconds() + cache_ttl,
                },
            );
        });
    }
}
