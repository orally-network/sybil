use candid::{CandidType, Nat};
use ic_cdk::update;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

use crate::{
    log,
    types::{
        feeds::{Feed, FeedError, FeedStorage},
        whitelist::WhitelistError,
    },
    utils::{validate_caller, validation, CallerError},
};

#[derive(Error, Debug)]
pub enum DefaultFeedError {
    #[error("Whitelist error: {0}")]
    WhitelistError(#[from] WhitelistError),
    #[error("Feed error: {0}")]
    FeedError(#[from] FeedError),
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
    #[error("Feed already exists")]
    FeedAlreadyExists,
    #[error("Feed not found")]
    FeedNotFound,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateDefaultFeedRequest {
    #[validate(regex = "validation::FEED_ID_REGEX")]
    pub id: String,
    pub decimals: Nat,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
}

#[update]
pub async fn create_default_feed(req: CreateDefaultFeedRequest) -> Result<(), String> {
    _create_default_feed(req)
        .await
        .map_err(|err| format!("failed to add a feed: {err}"))
}

async fn _create_default_feed(req: CreateDefaultFeedRequest) -> Result<(), DefaultFeedError> {
    validate_caller()?;
    if FeedStorage::contains(&req.id) {
        return Err(DefaultFeedError::FeedAlreadyExists);
    }

    let feed = Feed::from(req.clone());

    FeedStorage::get_default_rate(&feed, None).await?;
    FeedStorage::add(feed);

    log!("[FEEDS] default feed added. Feed ID: {}", req.id);
    Ok(())
}

#[update]
pub async fn remove_default_feed(id: String) -> Result<(), String> {
    _remove_default_feed(id)
        .await
        .map_err(|err| format!("failed to remove a feed: {err}"))
}

async fn _remove_default_feed(id: String) -> Result<(), DefaultFeedError> {
    validate_caller()?;
    if !FeedStorage::contains(&id) {
        return Err(DefaultFeedError::FeedNotFound);
    }

    FeedStorage::remove(&id);

    log!("[FEEDS] default feed removed. Feed ID: {}", id);
    Ok(())
}
