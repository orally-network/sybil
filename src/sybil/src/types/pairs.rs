use std::collections::HashMap;

use candid::Nat;
use ic_cdk::{
    api::management_canister::http_request::CanisterHttpRequestArgument,
    export::{
        candid::CandidType,
        serde::{Deserialize, Serialize},
    },
};
use ic_web3_rs::futures::future::join_all;
use jsonptr::{Pointer, Resolve};
use serde_json::Value;
use thiserror::Error;
use validator::Validate;

use super::{
    balances::{BalanceError, Balances},
    cache::{HttpCache, HttpCacheError},
    exchange_rate::{self, Asset, AssetClass, ExchangeRateError, GetExchangeRateRequest},
    rate_data::{RateDataError, RateDataLight},
    state, Address, Seconds, Timestamp,
};
use crate::{
    clone_with_state, defer,
    jobs::cache_cleaner,
    methods::{custom_pairs::CreateCustomPairRequest, default_pairs::CreateDefaultPairRequest},
    utils::{canister, nat, siwe::SiweError, time, validation, vec},
    CACHE, STATE,
};

const MIN_EXPECTED_BYTES: u64 = 1;
const MAX_EXPECTED_BYTES: u64 = 1024 * 1024 * 2;

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
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?
            .as_str()
            .ok_or(HttpCacheError::InvalidResponseBodyResolver(
                "value is not str".into(),
            ))?;

        Ok((data.into(), cached_at))
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
                PairType::Default => Self::get_default_rate(&pair).await,
                PairType::Custom { sources, .. } => Self::get_custom_rate(&pair, &sources).await,
            },
            None => Err(PairError::PairNotFound),
        }?;

        if with_signature {
            rate.sign().await?;
        }

        Ok(rate)
    }

    pub async fn get_default_rate(pair: &Pair) -> Result<RateDataLight, PairError> {
        if let Some(cache) = CACHE.with(|cache| cache.borrow_mut().get_entry(&pair.id)) {
            return Ok(cache);
        }

        let (base_asset, quote_asset) =
            Self::get_assets(&pair.id).ok_or(PairError::InvalidPairId)?;
        let timestamp = time::in_seconds();

        let req = GetExchangeRateRequest {
            base_asset,
            quote_asset,
            timestamp: Some(timestamp),
        };

        let exchange_rate_canister =
            exchange_rate::Service(clone_with_state!(exchange_rate_canister));

        let exchange_rate = Result::<_, _>::from(
            exchange_rate_canister
                .get_exchange_rate(req)
                .await
                .map_err(|(_, msg)| PairError::UnableToGetRate(msg))?
                .0,
        )?;

        let rate_data = RateDataLight {
            symbol: pair.id.clone(),
            rate: exchange_rate.rate,
            decimals: exchange_rate.metadata.decimals as u64,
            timestamp,
            ..Default::default()
        };

        CACHE.with(|cache| {
            cache
                .borrow_mut()
                .add_entry(pair.id.clone(), rate_data.clone(), pair.update_freq)
        });

        Ok(rate_data)
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
}
