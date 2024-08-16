use crate::{
    log,
    methods::balances,
    types::{
        balances::Balances,
        feed_types::get_multiple_asset_data::{
            GetMultipleAssetDataMetadata, GetMultipleAssetDataResult,
        },
        feeds::FeedStorage,
    },
    utils::{canister, convertion::convert_usd_to_eth, time::in_seconds},
};
use candid::Nat;
use futures::future::join_all;
use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::{
    stringify_func_call,
    types::{cache::Cache, feed_types::rate_data::MultipleAssetsDataResult, feeds::FeedError},
    utils::siwe,
};

use super::get_asset_data::_get_asset_data;

#[update]
pub async fn get_multiple_assets_data(
    ids: Vec<String>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<MultipleAssetsDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_multiple_assets_data(ids, false, None)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))
}

#[update]
pub async fn get_multiple_assets_data_with_proof(
    ids: Vec<String>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<MultipleAssetsDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let mut multiple_assetds_data = _get_multiple_assets_data(ids, true, None)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))?;

    multiple_assetds_data
        .sign()
        .await
        .map_err(|e| format!("failed to sign: {}", e))?;

    Ok(multiple_assetds_data)
}

#[inline]
#[cycles_count]
pub async fn _get_multiple_assets_data(
    ids: Vec<String>,
    with_signature: bool,
    payer: Option<String>,
) -> Result<MultipleAssetsDataResult, FeedError> {
    let func_signature = stringify_func_call!(_get_multiple_assets_data(ids, with_signature));

    let func_body = async move {
        let mut data = Vec::with_capacity(ids.len());

        let futures = ids
            .into_iter()
            .map(|id| _get_asset_data(id, false, payer.clone()))
            .collect::<Vec<_>>();

        for result in join_all(futures).await {
            data.push(result?.data);
        }

        let mut rates = MultipleAssetsDataResult {
            data,
            signature: None,
        };

        if with_signature {
            rates.sign().await.map_err(FeedError::RateDataError)?;
        }

        Ok(rates)
    };

    Cache::with(func_signature, func_body).evaluate().await
}

#[cycles_count]
pub async fn _get_multiple_assets_data_result(
    ids: Vec<String>,
    with_signature: bool,
    payer: Option<String>,
) -> Result<GetMultipleAssetDataResult, FeedError> {
    let mut data = Vec::with_capacity(ids.len());

    let futures = ids
        .iter()
        .map(|id| FeedStorage::rate(&id, false, None))
        .collect::<Vec<_>>();

    let mut cost = Nat::from(0);

    for result in join_all(futures).await {
        let result = result?;
        data.push(result.1.data);
        cost += result.0
    }

    let mut result = GetMultipleAssetDataResult {
        data,
        meta: Some(GetMultipleAssetDataMetadata {
            ids,
            timestamp: in_seconds(),
            fee: 0.into(),
            fee_symbol: "ETH".to_string(), // TODO: it's hardcoded, change it properly
        }),
        signature: None,
        bytes: None,
    };

    if payer.is_none() {
        let fee = convert_usd_to_eth(cost.clone(), balances::DECIMALS).await?;
        result.meta.as_mut().map(|meta| meta.fee = fee);
    }

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &cost)?;
        Balances::add_amount(&canister::eth_address().await?, &cost)?;
    }

    Ok(result)
}
