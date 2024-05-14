use crate::log;
use futures::future::join_all;
use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::{
    stringify_func_call,
    types::{cache::Cache, feeds::FeedError, rate_data::MultipleAssetsDataResult},
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

    _get_multiple_assets_data(ids, false, None, None)
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

    let mut multiple_assetds_data = _get_multiple_assets_data(ids, true, None, None)
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
    cache_ttl: Option<u64>,
) -> Result<MultipleAssetsDataResult, FeedError> {
    let func_signature = stringify_func_call!(_get_multiple_assets_data(ids, with_signature));

    let func_body = async move {
        let mut data = Vec::with_capacity(ids.len());

        let futures = ids
            .into_iter()
            .map(|id| _get_asset_data(id, false, payer.clone(), None))
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

    Cache::with(func_signature, func_body, cache_ttl).await
}
