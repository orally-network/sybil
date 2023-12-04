use std::{collections::HashMap, time::Duration};

use candid::{CandidType, Nat};
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
use ic_web3_rs::futures::future::join_all;
use jsonptr::{Pointer, Resolve};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use validator::Validate;

use super::{
    balances::{BalanceError, Balances},
    cache::{HttpCache, HttpCacheError},
    exchange_rate::{Asset, AssetClass, ExchangeRate, ExchangeRateError, GetExchangeRateRequest},
    rate_data::{RateDataError, RateDataLight},
    state, Address, Seconds, Timestamp,
};
use crate::{
    clone_with_state, defer,
    jobs::cache_cleaner,
    log,
    methods::{custom_pairs::CreateCustomPairRequest, default_pairs::CreateDefaultPairRequest},
    types::exchange_rate::Service,
    utils::{canister, nat, siwe::SiweError, sleep, time, validation, vec},
    CACHE, STATE,
};

const MIN_EXPECTED_BYTES: u64 = 1;
const MAX_EXPECTED_BYTES: u64 = 1024 * 1024 * 2;
const RATE_FETCH_DEFAULT_XRC_MAX_RETRIES: u64 = 5;
const RATE_FETCH_FALLBACK_XRC_MAX_RETRIES: u64 = 5;
const WAITING_BEFORE_RETRY_MS: Duration = Duration::from_millis(500);

#[derive(Error, Debug)]
pub enum PairError {
    #[error("Pair not found")]
    PairNotFound,
    #[error("Invalid pair id")]
    InvalidPairId,
    #[error("Unable to get rate: {0}")]
    UnableToGetRate(String),
    #[error("Exchange rate canister error: {0}")]
    ExchangeRateCanisterError(#[from] ExchangeRateError),
    #[error("No rate value got from sources")]
    NoRateValueGotFromSources,
    #[error("Rate data error: {0}")]
    RateDataError(#[from] RateDataError),
    #[error("SIWE error: {0}")]
    SIWEError(#[from] SiweError),
    #[error("Balance error: {0}")]
    Balance(#[from] BalanceError),
    #[error("Canister error: {0}")]
    Canister(#[from] canister::CanisterError),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct Source {
    #[validate(url)]
    pub uri: String,
    #[validate(regex = "validation::RATE_RESOLVER")]
    pub resolver: String,
    #[validate(range(min = "MIN_EXPECTED_BYTES", max = "MAX_EXPECTED_BYTES"))]
    pub expected_bytes: u64,
}

impl Source {
    pub async fn rate(&self, expr_freq: Seconds) -> Result<(u64, Seconds), HttpCacheError> {
        let req = CanisterHttpRequestArgument {
            url: self.uri.clone(),
            max_response_bytes: Some(self.expected_bytes),
            ..Default::default()
        };

        defer!(cache_cleaner::execute());
        let (response, cached_at) = HttpCache::request_with_access(&req, expr_freq).await?;

        let ptr = Pointer::try_from(self.resolver.clone())
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?;

        let data = serde_json::from_slice::<Value>(&response.body)?;

        let rate = data
            .resolve(&ptr)
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?
            .as_u64()
            .ok_or(HttpCacheError::InvalidResponseBodyResolver(
                "value is not number".into(),
            ))?;

        Ok((rate, cached_at))
    }

    pub async fn data(&self, expr_freq: Seconds) -> Result<(String, Seconds), HttpCacheError> {
        let req = CanisterHttpRequestArgument {
            url: self.uri.clone(),
            max_response_bytes: Some(self.expected_bytes),
            ..Default::default()
        };

        defer!(cache_cleaner::execute());
        let (response, cached_at) = HttpCache::request_with_access(&req, expr_freq).await?;

        let ptr = Pointer::try_from(self.resolver.clone())
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?;

        let json = serde_json::from_slice::<Value>(&response.body)?;

        let data = json
            .resolve(&ptr)
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?;

        Ok((format!("{data}"), cached_at))
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub enum PairType {
    Custom {
        sources: Vec<Source>,
    },
    #[default]
    Default,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct PairStatus {
    last_update: Timestamp,
    updated_counter: u64,
    requests_counter: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Pair {
    pub id: String,
    pub pair_type: PairType,
    pub update_freq: Seconds,
    pub decimals: u64,
    pub status: PairStatus,
    pub owner: Address,
    pub data: Option<RateDataLight>,
}

impl Pair {
    pub fn set_owner(&mut self, owner: Address) {
        self.owner = owner;
    }

    pub fn shrink_sources(&mut self) {
        if let PairType::Custom { sources } = &mut self.pair_type {
            sources.retain(|source| source.expected_bytes > 0);
        }
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct PairsStorage(HashMap<String, Pair>);

impl From<CreateCustomPairRequest> for Pair {
    fn from(req: CreateCustomPairRequest) -> Self {
        Self {
            id: req.pair_id,
            pair_type: PairType::Custom {
                sources: req.sources,
            },
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}

impl From<CreateDefaultPairRequest> for Pair {
    fn from(req: CreateDefaultPairRequest) -> Self {
        Self {
            id: req.pair_id,
            pair_type: PairType::Default,
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}

impl PairsStorage {
    pub fn add(pair: Pair) {
        STATE.with(|state| {
            state.borrow_mut().pairs.0.insert(pair.id.clone(), pair);
        })
    }

    pub fn remove(pair_id: &str) {
        STATE.with(|state| {
            state.borrow_mut().pairs.0.remove(pair_id);
        })
    }

    pub async fn rate(pair_id: &str, with_signature: bool) -> Result<RateDataLight, PairError> {
        let mut rate = match Self::get(pair_id) {
            Some(pair) => match pair.pair_type.clone() {
                PairType::Default => {
                    log!("[PAIRS] default pair requested: pair ID: {}", pair_id);
                    Self::get_default_rate(&pair).await
                }
                PairType::Custom { sources, .. } => {
                    log!(
                        "[PAIRS] cusom pair requested: pair ID: {}, sources: {:#?}",
                        pair_id,
                        sources
                    );
                    Self::get_custom_rate(&pair, &sources).await
                }
            },
            None => Err(PairError::PairNotFound),
        }?;

        if with_signature {
            rate.sign().await?;
        }

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let pair = state
                .pairs
                .0
                .get_mut(pair_id)
                .ok_or(PairError::PairNotFound)?;

            pair.data = Some(rate.clone());

            Result::<(), PairError>::Ok(())
        })?;

        log!("[PAIRS] requested rate: {:#?}", rate);

        Ok(rate)
    }

    pub async fn get_default_rate(pair: &Pair) -> Result<RateDataLight, PairError> {
        if let Some(cache) = CACHE.with(|cache| cache.borrow_mut().get_entry(&pair.id)) {
            log!("[PAIRS] get_default_rate found pair in cache");
            return Ok(cache);
        }

        let (base_asset, quote_asset) =
            Self::get_assets(&pair.id).ok_or(PairError::InvalidPairId)?;
        let req = GetExchangeRateRequest {
            base_asset,
            quote_asset,
            timestamp: None,
        };

        let xrc = Service(clone_with_state!(exchange_rate_canister));

        let exchange_rate = match Self::call_xrc_with_attempts(
            xrc,
            req.clone(),
            RATE_FETCH_DEFAULT_XRC_MAX_RETRIES,
        )
        .await
        {
            Ok(exchange_rate) => exchange_rate,
            Err(err) => {
                log!(
                    "[PAIRS] get_default_rate got error from default xrc: {}",
                    err
                );

                let xrc = Service(clone_with_state!(fallback_xrc));

                Self::call_xrc_with_attempts(xrc, req.clone(), RATE_FETCH_FALLBACK_XRC_MAX_RETRIES)
                    .await?
            }
        };

        let rate_data = RateDataLight {
            symbol: pair.id.clone(),
            rate: exchange_rate.rate,
            decimals: pair.decimals,
            timestamp: exchange_rate.timestamp,
            ..Default::default()
        };

        CACHE.with(|cache| {
            cache
                .borrow_mut()
                .add_entry(pair.id.clone(), rate_data.clone(), pair.update_freq);
        });

        Ok(rate_data)
    }

    async fn call_xrc_with_attempts(
        exchange_rate_canister: Service,
        mut req: GetExchangeRateRequest,
        max_attempts: u64,
    ) -> Result<ExchangeRate, PairError> {
        let mut exchange_rate = ExchangeRate::default();
        for attempt in 0..(max_attempts) {
            req.timestamp = Some(time::in_seconds() - 5);

            log!(
                "[PAIRS] get_default_rate requests xrc: attempt: {}, req: {:#?}",
                attempt,
                req
            );

            let exchange_rate_result = Result::<_, _>::from(
                exchange_rate_canister
                    .get_exchange_rate(req.clone())
                    .await
                    .map_err(|(_, msg)| PairError::UnableToGetRate(msg))?
                    .0,
            );

            match exchange_rate_result {
                Ok(_exchange_rate) => {
                    log!(
                        "[PAIRS] get_default_rate got response from xrc: {:?}",
                        _exchange_rate
                    );
                    exchange_rate = _exchange_rate;
                    break;
                }
                Err(ExchangeRateError::RateLimited) => {
                    return Err(PairError::ExchangeRateCanisterError(
                        ExchangeRateError::RateLimited,
                    ));
                }
                Err(err) => {
                    log!(
                        "[PAIRS] Exchange rate Error on attempt {}: {}",
                        attempt,
                        err,
                    );

                    sleep(WAITING_BEFORE_RETRY_MS).await;

                    if attempt == max_attempts - 1 {
                        return Err(PairError::ExchangeRateCanisterError(err));
                    }

                    continue;
                }
            };
        }

        return Ok(exchange_rate);
    }

    pub async fn get_custom_rate(
        pair: &Pair,
        sources: &[Source],
    ) -> Result<RateDataLight, PairError> {
        let canister_addr = canister::eth_address().await?;

        let bytes = Nat::from(
            sources
                .iter()
                .map(|source| source.expected_bytes)
                .sum::<u64>(),
        );
        let fee_per_byte = state::get_cfg().balances_cfg.fee_per_byte;
        let fee = fee_per_byte * bytes;

        if !Balances::is_sufficient(&pair.owner, &fee)? {
            return Err(BalanceError::InsufficientBalance)?;
        };

        let futures = sources
            .iter()
            .map(|source| source.rate(pair.update_freq))
            .collect::<Vec<_>>();

        let (mut results, cached_at_timestamps) = join_all(futures)
            .await
            .iter()
            .filter_map(|res| res.as_ref().ok().copied())
            .unzip::<_, _, Vec<_>, Vec<_>>();

        Balances::reduce_amount(&pair.owner, &fee)?;
        Balances::add_amount(&canister_addr, &fee)?;

        results.sort();

        let rate =
            *vec::find_most_frequent_value(&results).ok_or(PairError::NoRateValueGotFromSources)?;

        Ok(RateDataLight {
            symbol: pair.id.clone(),
            rate,
            decimals: pair.decimals,
            timestamp: cached_at_timestamps[0],
            ..Default::default()
        })
    }

    pub fn get(pair_id: &str) -> Option<Pair> {
        STATE.with(|state| state.borrow().pairs.0.get(pair_id).cloned())
    }

    pub fn get_assets(pair_id: &str) -> Option<(Asset, Asset)> {
        let assets: Vec<&str> = pair_id.split_terminator('/').collect();

        if let (Some(base_asset), Some(quote_asset)) = (assets.first(), assets.last()) {
            return Some((
                Asset {
                    class: AssetClass::Cryptocurrency,
                    symbol: base_asset.to_string(),
                },
                Asset {
                    class: AssetClass::FiatCurrency,
                    symbol: quote_asset.to_string(),
                },
            ));
        }

        None
    }

    pub fn contains(pair_id: &str) -> bool {
        STATE.with(|state| state.borrow().pairs.0.contains_key(pair_id))
    }

    pub fn pairs() -> Vec<Pair> {
        STATE.with(|state| state.borrow().pairs.0.values().cloned().collect())
    }

    pub fn clear() {
        STATE.with(|state| state.borrow_mut().pairs.0.clear());
    }
}
