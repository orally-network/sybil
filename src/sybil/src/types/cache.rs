use std::future::Future;
use std::{collections::HashMap, time::Duration};

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpResponse,
};
use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;

use crate::{
    clone_with_state,
    jobs::cache_cleaner,
    log,
    utils::{nat, time},
};

use super::{Seconds, Timestamp};

const HTTP_WAITING_DELAY_SECS: u64 = 3;
const HTTP_WATTING_TIMEOUT_SECS: u64 = 24;
const HTTP_OUTCALL_REQUEST_CYCLES: u128 = 400_000_000;
const HTTP_OUTCALL_PAYLOAD_CYCLES: u128 = 100_000;
const MAX_RESPONSE_BYTES: u128 = 2 * 1024 * 1024; // 2 MB

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct Cache {
    records: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
struct CacheEntry {
    expired_at: u64,
    data: Vec<u8>,
}

impl Cache {
    pub fn add_entry(&mut self, key: String, data: Vec<u8>) {
        let cache_expiratione = clone_with_state!(cache_expiration);

        let entry = CacheEntry {
            expired_at: Duration::from_nanos(ic_cdk::api::time()).as_secs() + cache_expiratione,
            data,
        };

        self.records.insert(key, entry);
    }

    pub fn get_entry(&mut self, key: &str) -> Option<Vec<u8>> {
        let entry = self.records.get(key);

        if let Some(entry) = entry {
            if entry.expired_at < Duration::from_nanos(ic_cdk::api::time()).as_secs() {
                self.records.remove(key);
                return None;
            }

            return Some(entry.data.clone());
        }

        None
    }

    pub async fn execute<'a, T, F, Fmt, Arg>(&mut self, key: &str, f: F, arg: Arg) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + 'a,
        F: FnOnce(Arg) -> Fmt,
        Fmt: Future<Output = Result<T>>,
    {
        let entry = self.get_entry(key);

        if let Some(entry) = entry {
            return Ok(serde_json::from_slice(&entry)?);
        }
        let result = f(arg).await?;
        let serialized = serde_json::to_vec(&result)?;
        self.add_entry(key.to_string(), serialized);

        Ok(result)
    }
}

#[async_trait]
pub trait HTTPCacheable {
    type HTTPCacheStats;
    type HTTPCacheError;

    fn expiration(&self) -> u64;
    async fn request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<HttpResponse, HTTPCacheError>;
    async fn force_request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<HttpResponse, HTTPCacheError>;
    fn clean(&mut self);
    fn stats(&self) -> Self::HTTPCacheStats;
}

#[derive(Default)]
pub struct HTTPCache {
    entries: HashMap<String, HTTPCacheEntry>,
    capacity: usize,
    stats: HTTPCacheStats,
}

#[derive(Default, Clone)]
struct HTTPCacheEntry {
    cached_at: Timestamp,
    // expiration frequency
    expr_freq: Seconds,
    response: Option<HttpResponse>,
}

impl HTTPCacheEntry {
    fn new(response: HttpResponse, expr_freq: Seconds) -> Self {
        Self {
            cached_at: time::in_seconds(),
            expr_freq,
            response: Some(response),
        }
    }

    fn is_expired(&self) -> bool {
        time::in_seconds() - self.cached_at > self.expr_freq
    }
}

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct HTTPCacheStats {
    hits: usize,
    misses: usize,
    cache_size: usize,
    total_requests: usize,
}

#[derive(Error, Debug)]
pub enum HTTPCacheError {
    #[error("HTTP outcall error with message: \"`{0}`\"")]
    HTTPOutcallError(String),
    #[error("Got error from server: \"`{0}`\"")]
    ServerError(String),
}

#[async_trait]
impl HTTPCacheable for HTTPCache {
    type HTTPCacheStats = HTTPCacheStats;
    type HTTPCacheError = HTTPCacheError;

    fn expiration(&self) -> u64 {
        clone_with_state!(cache_expiration)
    }

    async fn request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<HttpResponse, HTTPCacheError> {
        log!("[HTTP CACHE] got request");
        self.stats.total_requests += 1;
        if self.entries.len() >= self.capacity {
            cache_cleaner::execute();
        }

        if let Some(entry) = self.entries.get(&request.url) {
            log!("[HTTP CACHE] record found in cache");
            if let Some(response) = &entry.response {
                log!("[HTTP CACHE] response found in cache");
                if entry.is_expired() {
                    log!("[HTTP CACHE] response expired");
                    return self.force_request(request, expr_freq).await;
                }
                log!("[HTTP CACHE] response not expired");
                return Ok(response.clone());
            }

            log!("[HTTP CACHE] response not found in cache");
            // waiting for the pending request to finish
            for _ in 0..(HTTP_WATTING_TIMEOUT_SECS / HTTP_WAITING_DELAY_SECS) {
                time::wait(HTTP_WAITING_DELAY_SECS).await;

                if let Some(response) = &entry.response {
                    return Ok(response.clone());
                }
            }
        }

        log!("[HTTP CACHE] record not found in cache");
        self.force_request(request, expr_freq).await
    }

    async fn force_request(
        &mut self,
        request: &CanisterHttpRequestArgument,
        expr_freq: Seconds,
    ) -> Result<HttpResponse, HTTPCacheError> {
        let mut cycles =
            HTTP_OUTCALL_REQUEST_CYCLES + (MAX_RESPONSE_BYTES * HTTP_OUTCALL_PAYLOAD_CYCLES);
        if let Some(body) = &request.body {
            cycles += body.len() as u128 * HTTP_OUTCALL_PAYLOAD_CYCLES;
        }

        self.entries
            .insert(request.url.clone(), HTTPCacheEntry::default());

        let (response,) = http_request(request.clone(), cycles)
            .await
            .map_err(|(_, msg)| {
                self.stats.misses += 1;
                HTTPCacheError::HTTPOutcallError(msg)
            })?;

        if nat::to_u64(&response.status) >= 400 {
            let msg = String::from_utf8(response.body).unwrap_or("unknown error".to_string());

            self.stats.misses += 1;

            return Err(HTTPCacheError::ServerError(msg));
        }

        let entry = self.entries.get_mut(&request.url);
        if let Some(entry) = entry {
            entry.response = Some(response.clone());
            entry.cached_at = time::in_seconds();
        } else {
            self.entries.insert(
                request.url.clone(),
                HTTPCacheEntry::new(response.clone(), expr_freq),
            );
        }

        self.stats.hits += 1;

        Ok(response)
    }

    fn clean(&mut self) {
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

    fn stats(&self) -> Self::HTTPCacheStats {
        let mut stats = self.stats.clone();
        stats.cache_size = self.entries.len();
        stats
    }
}
