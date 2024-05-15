use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::log;

use crate::types::feed_types::rate_data::AssetDataResult;
use crate::{
    metrics, stringify_func_call,
    types::{
        cache::Cache,
        feeds::{FeedError, FeedStorage},
    },
    utils::siwe,
};

#[update]
pub async fn get_asset_data(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_asset_data(id, false, None, None)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[update]
pub async fn get_asset_data_with_proof(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_asset_data(id, true, Some(payer), None)
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

#[inline]
#[cycles_count]
pub async fn _get_asset_data(
    id: String,
    with_signature: bool,
    payer: Option<String>,
    cache_ttl: Option<u64>,
) -> Result<AssetDataResult, FeedError> {
    if with_signature {
        metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc GET_ASSET_DATA_CALLS, id);
    }

    let func_signature = stringify_func_call!(_get_asset_data(id, with_signature));

    let rate = Cache::with(
        func_signature,
        FeedStorage::rate(&id, with_signature, payer),
        cache_ttl,
    )
    .await?;

    if with_signature {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
    }
    Ok(rate)
}
