use std::time::Duration;

use super::{
    exchange_rate,
    exchange_rate::{Asset, AssetClass, GetExchangeRateRequest, GetExchangeRateResult},
};
use crate::{
    types::{rate_data::RateDataLight, PairType},
    utils::PairMetadata,
    CACHE, STATE,
};
use anyhow::{anyhow, Context, Result};
use ic_cdk::{
    api::management_canister::http_request::{
        http_request, CanisterHttpRequestArgument, HttpMethod,
    },
    export::Principal,
};
use jsonptr::{Pointer, Resolve};
use serde_json::Value;
use url::Url;

pub async fn get_rate(pair_metadata: PairMetadata, with_signature: bool) -> Result<RateDataLight> {
    match pair_metadata.pair_type {
        PairType::CustomPair => get_rate_from_custom_pair(pair_metadata, with_signature).await,
        PairType::Pair => get_rate_from_pair(pair_metadata, with_signature).await,
    }
}

async fn get_rate_from_custom_pair(pair_metadata: PairMetadata, with_signature: bool) -> Result<RateDataLight> {
    let (rate, source) = STATE.with(|state| {
        let state = state.borrow();

        let custom_pair = state
            .custom_pairs
            .get(pair_metadata.index)
            .expect("custom pair index should exists");

        if custom_pair.available_executions == 0 {
            return (Some(custom_pair.data.clone()), custom_pair.source.clone());
        }

        if (custom_pair.last_update + custom_pair.frequency) < Duration::from_nanos(ic_cdk::api::time()).as_secs() {
            return (None, custom_pair.source.clone());
        };

        (Some(custom_pair.data.clone()), custom_pair.source.clone())
    });

    if let Some(rate) = rate {
        return Ok(rate);
    }

    let url = Url::parse(&source.uri)?;
    let (rate, _) =
        get_custom_rate_with_cache(&url, &source.resolver, &pair_metadata.pair_id, false, with_signature).await?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        let mut custom_pair = state
            .custom_pairs
            .get_mut(pair_metadata.index)
            .expect("custom pair index should exists");

        custom_pair.data = rate.clone();
        custom_pair.last_update = Duration::from_nanos(ic_cdk::api::time()).as_secs();
    });

    Ok(rate)
}

pub async fn get_rate_from_pair(pair_metadata: PairMetadata, with_signature: bool) -> Result<RateDataLight> {
    let rate = STATE.with(|state| {
        let state = state.borrow();

        let pair = state
            .pairs
            .get(pair_metadata.index)
            .expect("custom pair index should exists");

        if (pair.last_update + pair.frequency) < Duration::from_nanos(ic_cdk::api::time()).as_secs() {
            return None;
        };

        Some(pair.data.clone())
    });

    if let Some(rate) = rate {
        return Ok(rate);
    }

    let rate = get_rate_with_cache(&pair_metadata.pair_id, with_signature).await?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        let mut pair = state
            .pairs
            .get_mut(pair_metadata.index)
            .expect("custom pair index should exists");

        pair.data = rate.clone();
        pair.last_update = Duration::from_nanos(ic_cdk::api::time()).as_secs();
    });

    Ok(rate)
}

pub async fn get_custom_rate_with_cache(
    url: &Url,
    resolver: &str,
    pair_id: &str,
    init: bool,
    with_signature: bool,
) -> Result<(RateDataLight, u64)> {
    let data = CACHE.with(|cache| cache.borrow_mut().get_entry(pair_id));

    if let Some(data) = data {
        return Ok((serde_json::from_slice(&data)?, data.len() as u64));
    }

    let request_args = CanisterHttpRequestArgument {
        url: url.to_string(),
        method: HttpMethod::GET,
        max_response_bytes: None,
        headers: vec![],
        body: None,
        transform: None,
    };

    let (response,) = http_request(request_args)
        .await
        .map_err(|(code, msg)| anyhow!("Failed to make a request: {}, {:?}", msg, code))?;

    let data: Value = serde_json::from_slice(&response.body)?;

    let ptr = Pointer::try_from(resolver).map_err(|err| anyhow!("invalid resolver: {err}"))?;

    let rate = data
        .resolve(&ptr)
        .map_err(|err| anyhow!("invalid resolver: {err}"))?
        .as_u64()
        .context("invalid resolver")?;

    let timestamp = Duration::from_nanos(ic_cdk::api::time()).as_secs();

    let mut rate_data = RateDataLight {
        symbol: pair_id.into(),
        rate,
        decimals: 0,
        timestamp,
        signature: None
    };

    if with_signature {
        rate_data.sign().await?;
    }

    let data_for_cache = serde_json::to_vec(&rate_data)?;

    CACHE.with(|cache| cache.borrow_mut().add_entry(pair_id.into(), data_for_cache));
    
    if !init {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let mut pair = state
                .custom_pairs
                .iter_mut()
                .find(|p| p.id == pair_id)
                .expect("pair should exist");

            pair.available_executions -= 1;
        });
    }

    Ok((rate_data, response.body.len() as u64))
}

pub async fn get_rate_with_cache(pair_id: &str, with_signature: bool) -> Result<RateDataLight> {
    let data = CACHE.with(|cache| cache.borrow_mut().get_entry(pair_id));

    if let Some(data) = data {
        return Ok(serde_json::from_slice(&data)?);
    }

    let exchange_rate_canister_id =
        STATE.with(|state| Principal::from_text(&state.borrow().exchange_rate_canister))?;

    let exchange_rate_canister = exchange_rate::Service(exchange_rate_canister_id);

    let assets: Vec<&str> = pair_id.split_terminator('/').collect();

    let base_asset = Asset {
        class: AssetClass::Cryptocurrency,
        symbol: assets
            .first()
            .expect("base asset symbol should exist")
            .to_string(),
    };

    let quote_asset = Asset {
        class: AssetClass::FiatCurrency,
        symbol: assets
            .last()
            .expect("quote asset symbol should exist")
            .to_string(),
    };

    let request = GetExchangeRateRequest {
        base_asset,
        quote_asset,
        timestamp: None,
    };

    let (rate_response,) = exchange_rate_canister
        .get_exchange_rate(request)
        .await
        .map_err(|(code, msg)| anyhow!("Failed to make a request: {}, {:?}", msg, code))?;

    let exchange_rate = match rate_response {
        GetExchangeRateResult::Ok(rate) => Ok(rate),
        GetExchangeRateResult::Err(err) => Err(err),
    }?;

    let mut rate = RateDataLight {
        symbol: pair_id.into(),
        rate: exchange_rate.rate,
        decimals: exchange_rate.metadata.decimals as u64,
        timestamp: Duration::from_nanos(ic_cdk::api::time()).as_secs(),
        signature: None
    };

    if with_signature {
        rate.sign().await?;
    }

    let data_for_cache = serde_json::to_vec(&rate)?;

    CACHE.with(|cache| cache.borrow_mut().add_entry(pair_id.into(), data_for_cache));

    Ok(rate)
}
