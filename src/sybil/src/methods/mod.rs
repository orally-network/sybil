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
    metrics,
    types::{
        feeds::{Feed, FeedError, FeedStorage, GetFeedsFilter},
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
pub async fn get_asset_data_with_proof(id: String) -> Result<AssetDataResult, String> {
    _get_asset_data_with_proof(id)
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

pub async fn _get_asset_data_with_proof(id: String) -> Result<AssetDataResult, AssetsError> {
    metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    let rate = FeedStorage::rate(&id, true).await?;

    metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    Ok(rate)
}

#[update]
pub async fn get_asset_data(id: String) -> Result<AssetDataResult, String> {
    _get_asset_data(id)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

async fn _get_asset_data(id: String) -> Result<AssetDataResult, AssetsError> {
    metrics!(inc GET_ASSET_DATA_CALLS, id);
    let mut rate = FeedStorage::rate(&id, false).await?;

    rate.signature = None;

    metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
    Ok(rate)
}

#[update]
pub async fn get_multiple_assets_data_with_proof(
    ids: Vec<String>,
) -> Result<MultipleAssetsDataResult, String> {
    let mut multiple_assetds_data = _get_multiple_assets_data(ids)
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
) -> Result<MultipleAssetsDataResult, String> {
    _get_multiple_assets_data(ids)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))
}

async fn _get_multiple_assets_data(
    ids: Vec<String>,
) -> Result<MultipleAssetsDataResult, AssetsError> {
    let mut data = Vec::with_capacity(ids.len());

    let futures = ids.into_iter().map(_get_asset_data).collect::<Vec<_>>();

    for result in join_all(futures).await {
        data.push(result?.data);
    }

    Ok(MultipleAssetsDataResult {
        data,
        signature: None,
    })
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
