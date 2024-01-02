use std::{collections::HashMap, time::Duration};

use candid::{CandidType, Nat};
use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpHeader};
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
    methods::{custom_feeds::CreateCustomFeedRequest, default_feeds::CreateDefaultFeedRequest},
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
pub enum FeedError {
    #[error("Pair not found")]
    FeedNotFound,
    #[error("Invalid pair id")]
    InvalidFeedId,
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
            headers: Self::get_default_headers(),
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

    pub fn get_default_headers() -> Vec<HttpHeader> {
        vec![
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
            HttpHeader {
                name: "User-Agent".to_string(),
                value: "sybil".to_string(),
            },
        ]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, CandidType)]
pub struct GetFeedsFilter {
    pub feed_type: Option<FeedType>,
    pub owner: Option<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub enum FeedType {
    Custom {
        sources: Vec<Source>,
    },
    #[default]
    Default,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct FeedStatus {
    last_update: Timestamp,
    updated_counter: u64,
    requests_counter: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Feed {
    pub id: String,
    pub feed_type: FeedType,
    pub update_freq: Seconds,
    pub decimals: u64,
    pub status: FeedStatus,
    pub owner: Address,
    pub data: Option<RateDataLight>,
}

impl Feed {
    pub fn set_owner(&mut self, owner: Address) {
        self.owner = owner;
    }

    pub fn shrink_sources(&mut self) {
        if let FeedType::Custom { sources } = &mut self.feed_type {
            sources.retain(|source| source.expected_bytes > 0);
        }
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct FeedStorage(HashMap<String, Feed>);

impl From<CreateCustomFeedRequest> for Feed {
    fn from(req: CreateCustomFeedRequest) -> Self {
        Self {
            id: req.feed_id,
            feed_type: FeedType::Custom {
                sources: req.sources,
            },
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}

impl From<CreateDefaultFeedRequest> for Feed {
    fn from(req: CreateDefaultFeedRequest) -> Self {
        Self {
            id: req.feed_id,
            feed_type: FeedType::Default,
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}

impl FeedStorage {
    pub fn add(feed: Feed) {
        STATE.with(|state| {
            state.borrow_mut().feeds.0.insert(feed.id.clone(), feed);
        })
    }

    pub fn remove(feed_id: &str) {
        STATE.with(|state| {
            state.borrow_mut().feeds.0.remove(feed_id);
        })
    }

    pub async fn rate(feed_id: &str, with_signature: bool) -> Result<RateDataLight, FeedError> {
        let mut rate = match Self::get(feed_id) {
            Some(feed) => match feed.feed_type.clone() {
                FeedType::Default => {
                    log!("[FEEDS] default feed requested: feed ID: {}", feed_id);
                    Self::get_default_rate(&feed).await
                }
                FeedType::Custom { sources, .. } => {
                    log!(
                        "[FEEDS] cusom feed requested: feed ID: {}, sources: {:#?}",
                        feed_id,
                        sources
                    );
                    Self::get_custom_rate(&feed, &sources).await
                }
            },
            None => Err(FeedError::FeedNotFound),
        }?;

        if with_signature {
            rate.sign().await?;
        }

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let feed = state
                .feeds
                .0
                .get_mut(feed_id)
                .ok_or(FeedError::FeedNotFound)?;

            feed.data = Some(rate.clone());

            Result::<(), FeedError>::Ok(())
        })?;

        log!("[FEEDS] requested rate: {:#?}", rate);

        Ok(rate)
    }

    pub async fn get_default_rate(feed: &Feed) -> Result<RateDataLight, FeedError> {
        if let Some(cache) = CACHE.with(|cache| cache.borrow_mut().get_entry(&feed.id)) {
            log!("[FEEDS] get_default_rate found feed in cache");
            return Ok(cache);
        }

        let (base_asset, quote_asset) =
            Self::get_assets(&feed.id).ok_or(FeedError::InvalidFeedId)?;
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
                    "[FEEDS] get_default_rate got error from default xrc: {}",
                    err
                );

                let xrc = Service(clone_with_state!(fallback_xrc));

                Self::call_xrc_with_attempts(xrc, req.clone(), RATE_FETCH_FALLBACK_XRC_MAX_RETRIES)
                    .await?
            }
        };

        let rate_data = RateDataLight {
            symbol: feed.id.clone(),
            rate: exchange_rate.rate,
            decimals: feed.decimals,
            timestamp: exchange_rate.timestamp,
            ..Default::default()
        };

        CACHE.with(|cache| {
            cache
                .borrow_mut()
                .add_entry(feed.id.clone(), rate_data.clone(), feed.update_freq);
        });

        Ok(rate_data)
    }

    async fn call_xrc_with_attempts(
        exchange_rate_canister: Service,
        mut req: GetExchangeRateRequest,
        max_attempts: u64,
    ) -> Result<ExchangeRate, FeedError> {
        let mut exchange_rate = ExchangeRate::default();
        for attempt in 0..(max_attempts) {
            req.timestamp = Some(time::in_seconds() - 5);

            log!(
                "[FEEDS] get_default_rate requests xrc: attempt: {}, req: {:#?}",
                attempt,
                req
            );

            let exchange_rate_result = Result::<_, _>::from(
                exchange_rate_canister
                    .get_exchange_rate(req.clone())
                    .await
                    .map_err(|(_, msg)| FeedError::UnableToGetRate(msg))?
                    .0,
            );

            match exchange_rate_result {
                Ok(_exchange_rate) => {
                    log!(
                        "[FEEDS] get_default_rate got response from xrc: {:?}",
                        _exchange_rate
                    );
                    exchange_rate = _exchange_rate;
                    break;
                }
                Err(ExchangeRateError::RateLimited) => {
                    return Err(FeedError::ExchangeRateCanisterError(
                        ExchangeRateError::RateLimited,
                    ));
                }
                Err(err) => {
                    log!(
                        "[FEEDS] Exchange rate Error on attempt {}: {}",
                        attempt,
                        err,
                    );

                    sleep(WAITING_BEFORE_RETRY_MS).await;

                    if attempt == max_attempts - 1 {
                        return Err(FeedError::ExchangeRateCanisterError(err));
                    }

                    continue;
                }
            };
        }

        return Ok(exchange_rate);
    }

    pub async fn get_custom_rate(
        feed: &Feed,
        sources: &[Source],
    ) -> Result<RateDataLight, FeedError> {
        let canister_addr = canister::eth_address().await?;

        let bytes = Nat::from(
            sources
                .iter()
                .map(|source| source.expected_bytes)
                .sum::<u64>(),
        );
        let fee_per_byte = state::get_cfg().balances_cfg.fee_per_byte;
        let fee = fee_per_byte * bytes;

        if !Balances::is_sufficient(&feed.owner, &fee)? {
            return Err(BalanceError::InsufficientBalance)?;
        };

        let futures = sources
            .iter()
            .map(|source| source.rate(feed.update_freq))
            .collect::<Vec<_>>();

        let (mut results, cached_at_timestamps) = join_all(futures)
            .await
            .iter()
            .filter_map(|res| match res {
                Ok(res) => {
                    return Some(res.clone());
                }
                Err(err) => {
                    log!("[FEEDS] error while getting custom rate: {:?}", err);
                    None
                }
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        Balances::reduce_amount(&feed.owner, &fee)?;
        Balances::add_amount(&canister_addr, &fee)?;

        results.sort();

        let rate =
            *vec::find_most_frequent_value(&results).ok_or(FeedError::NoRateValueGotFromSources)?;

        Ok(RateDataLight {
            symbol: feed.id.clone(),
            rate,
            decimals: feed.decimals,
            timestamp: cached_at_timestamps[0],
            ..Default::default()
        })
    }

    pub fn get(feed_id: &str) -> Option<Feed> {
        STATE.with(|state| state.borrow().feeds.0.get(feed_id).cloned())
    }

    pub fn get_assets(feed_id: &str) -> Option<(Asset, Asset)> {
        let assets: Vec<&str> = feed_id.split_terminator('/').collect();

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

    pub fn contains(feed_id: &str) -> bool {
        STATE.with(|state| state.borrow().feeds.0.contains_key(feed_id))
    }

    pub fn feeds() -> Vec<Feed> {
        STATE.with(|state| state.borrow().feeds.0.values().cloned().collect())
    }

    pub fn clear() {
        STATE.with(|state| state.borrow_mut().feeds.0.clear());
    }
}
