pub mod allowances;
pub mod balances;
pub mod controllers;
pub mod custom_feeds;
pub mod default_feeds;
pub mod signatures;
pub mod transforms;
pub mod whitelist;

use futures::future::join_all;
use ic_cdk::{query, update};

use thiserror::Error;

use ic_utils::{
    api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
    get_information, update_information,
};

use crate::{
    log, metrics,
    types::{
        feeds::{Feed, FeedError, FeedStorage, GetFeedsFilter, DEFAULT_UPDATE_FREQ},
        pagination::{Pagination, PaginationResult},
        rate_data::{AssetDataResult, MultipleAssetsDataResult},
    },
    utils::{canister, siwe},
};

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Feed error: {0}")]
    FeedError(#[from] FeedError),
    #[error("Siwe error: {0}")]
    SiweError(#[from] siwe::SiweError),
}

#[query]
fn is_feed_exists(id: String) -> bool {
    FeedStorage::contains(&id)
}

#[query]
async fn get_feed(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<Option<Feed>, String> {
    _get_feed(id, msg, sig)
        .await
        .map_err(|e| format!("failed to get feed: {}", e))
}

async fn _get_feed(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<Option<Feed>, AssetsError> {
    let caller = if let (Some(msg), Some(sig)) = (msg, sig) {
        Some(siwe::recover(&msg, &sig).await?)
    } else {
        None
    };

    if let Some(mut feed) = FeedStorage::get(&id) {
        feed.censor_if_needed(&caller);
        Ok(Some(feed))
    } else {
        Ok(None)
    }
}

#[query]
async fn get_feeds(
    filter: Option<GetFeedsFilter>,
    pagination: Option<Pagination>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<PaginationResult<Feed>, String> {
    _get_feeds(filter, pagination, msg, sig)
        .await
        .map_err(|e| format!("failed to get feeds: {}", e))
}

async fn _get_feeds(
    filter: Option<GetFeedsFilter>,
    pagination: Option<Pagination>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<PaginationResult<Feed>, AssetsError> {
    let caller = if let (Some(msg), Some(sig)) = (msg, sig) {
        Some(siwe::recover(&msg, &sig).await?)
    } else {
        None
    };

    let mut feeds = FeedStorage::get_all(filter);

    feeds
        .iter_mut()
        .for_each(|feed| feed.censor_if_needed(&caller));

    match pagination {
        Some(pagination) => {
            feeds.sort_by(|l, r| l.id.cmp(&r.id));
            Ok(pagination.paginate(feeds))
        }
        None => Ok(feeds.into()),
    }
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

    _get_asset_data(id, true, payer)
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

pub async fn _get_asset_data(
    id: String,
    with_signature: bool,
    payer: String,
) -> Result<AssetDataResult, AssetsError> {
    if with_signature {
        metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc GET_ASSET_DATA_CALLS, id);
    }
    let rate = FeedStorage::rate(&id, with_signature, payer).await?;

    if with_signature {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
    }
    Ok(rate)
}

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

    _get_xrc_data(id, false, payer)
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

    _get_xrc_data(id, true, payer)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

pub async fn _get_xrc_data(
    id: String,
    with_signature: bool,
    payer: String,
) -> Result<AssetDataResult, AssetsError> {
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
}

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

    _get_asset_data(id, false, payer)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
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

    let mut multiple_assetds_data = _get_multiple_assets_data(ids, true, payer)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))?;

    multiple_assetds_data
        .sign()
        .await
        .map_err(|e| format!("failed to sign: {}", e))?;

    Ok(multiple_assetds_data)
}

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

    _get_multiple_assets_data(ids, false, payer)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))
}

pub async fn _get_multiple_assets_data(
    ids: Vec<String>,
    with_signature: bool,
    payer: String,
) -> Result<MultipleAssetsDataResult, AssetsError> {
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
}

#[query(name = "getCanistergeekInformation")]
pub async fn get_canistergeek_information(
    request: GetInformationRequest,
) -> GetInformationResponse<'static> {
    get_information(request)
}

#[update(name = "updateCanistergeekInformation")]
pub async fn update_canistergeek_information(request: UpdateInformationRequest) {
    update_information(request);
}

#[update]
pub async fn eth_address() -> Result<String, String> {
    canister::eth_address()
        .await
        .map_err(|e| format!("failed to get eth address: {}", e))
}
