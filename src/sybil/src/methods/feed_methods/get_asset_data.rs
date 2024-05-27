use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::log;

use crate::methods::balances;
use crate::types::balances::Balances;
use crate::types::feed_types::get_asset_data::{GetAssetDataMetadata, GetAssetDataResult};
use crate::types::feed_types::rate_data::AssetDataResult;
use crate::utils::canister;
use crate::utils::convertion::convert_usd_to_eth;
use crate::utils::time::in_seconds;
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
    let func_signature = stringify_func_call!(_get_asset_data(id, with_signature));

    let func_body = async move {
        if with_signature {
            metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
        } else {
            metrics!(inc GET_ASSET_DATA_CALLS, id);
        }

        let (cost, rate) = FeedStorage::rate(&id, with_signature, payer.clone()).await?;

        if let Some(payer) = payer {
            Balances::reduce_amount(&payer, &cost)?;
            Balances::add_amount(&canister::eth_address().await?, &cost)?;
        }

        if with_signature {
            metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
        } else {
            metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
        }
        Ok(rate)
    };

    Cache::with(func_signature, func_body, |_| {}, cache_ttl).await
}

#[cycles_count]
pub async fn _get_asset_data_result(
    id: String,
    with_signature: bool,
    payer: Option<String>,
) -> Result<GetAssetDataResult, FeedError> {
    if with_signature {
        metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc GET_ASSET_DATA_CALLS, id);
    }

    let (cost, rate) = FeedStorage::rate(&id, with_signature, payer.clone()).await?;

    let mut result = GetAssetDataResult {
        data: rate.data,
        meta: GetAssetDataMetadata {
            id: id.clone(),
            timestamp: in_seconds(),
            fee: 0.into(),
            fee_symbol: "ETH".to_string(), // TODO: it's hardcoded, change it properly
        },
        signature: None,
    };

    if payer.is_none() {
        let fee = convert_usd_to_eth(cost.clone(), balances::DECIMALS).await?;
        result.meta.fee = fee;
    }

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &cost)?;
        Balances::add_amount(&canister::eth_address().await?, &cost)?;
    }

    if with_signature {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
    }

    Ok(result)
}
