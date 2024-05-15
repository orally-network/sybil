use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::{
    log, stringify_func_call,
    types::{
        cache::Cache,
        feed_types::rate_data::AssetDataResult,
        feeds::{Feed, FeedError, FeedStorage, DEFAULT_UPDATE_FREQ},
    },
    utils::siwe,
};

#[update]
pub async fn get_xrc_data(
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

    _get_xrc_data(id, false, Some(payer), None)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[update]
pub async fn get_xrc_data_with_proof(
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

    _get_xrc_data(id, true, Some(payer), None)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[inline]
#[cycles_count]
pub async fn _get_xrc_data(
    id: String,
    with_signature: bool,
    payer: Option<String>,
    cache_ttl: Option<u64>,
) -> Result<AssetDataResult, FeedError> {
    let func_signature = stringify_func_call!(_get_xrc_data(id, with_signature));

    let func_body = async move {
        let rate = Feed {
            id: id.clone(),
            update_freq: DEFAULT_UPDATE_FREQ,
            ..Default::default()
        };

        let mut rate = FeedStorage::get_default_rate(&rate, None).await?;

        if with_signature {
            rate.sign().await.map_err(FeedError::RateDataError)?;
        }

        Ok(rate)
    };

    Cache::with(func_signature, func_body, cache_ttl).await
}
