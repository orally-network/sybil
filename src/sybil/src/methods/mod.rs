pub mod balances;
pub mod controllers;
pub mod custom_feeds;
pub mod data_fetchers;
pub mod default_feeds;
pub mod transforms;
pub mod whitelist;

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
        rate_data::RateDataLight,
    },
    utils::canister,
};

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Feed error: {0}")]
    FeedError(#[from] FeedError),
}

#[query]
fn is_feed_exists(feed_id: String) -> bool {
    FeedStorage::contains(&feed_id)
}

#[query]
fn get_feeds(
    filter: Option<GetFeedsFilter>,
    pagination: Option<Pagination>,
) -> PaginationResult<Feed> {
    let mut feeds = FeedStorage::get_all(filter);
    feeds.iter_mut().for_each(|feed| feed.shrink_sources());

    match pagination {
        Some(pagination) => {
            feeds.sort_by(|l, r| l.id.cmp(&r.id));
            pagination.paginate(feeds)
        }
        None => feeds.into(),
    }
}

#[update]
pub async fn get_asset_data_with_proof(feed_id: String) -> Result<RateDataLight, String> {
    _get_asset_data_with_proof(feed_id)
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

pub async fn _get_asset_data_with_proof(feed_id: String) -> Result<RateDataLight, AssetsError> {
    metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, feed_id);
    let rate = FeedStorage::rate(&feed_id, true).await?;

    metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, feed_id);
    Ok(rate)
}

#[update]
pub async fn get_asset_data(feed_id: String) -> Result<RateDataLight, String> {
    _get_asset_data(feed_id)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

async fn _get_asset_data(feed_id: String) -> Result<RateDataLight, AssetsError> {
    metrics!(inc GET_ASSET_DATA_CALLS, feed_id);
    let mut rate = FeedStorage::rate(&feed_id, false).await?;

    rate.signature = None;

    metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, feed_id);
    Ok(rate)
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
