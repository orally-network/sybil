use candid::{CandidType, Nat};
use ic_cdk::update;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::log;
use crate::metrics;
use crate::types::feeds::FeedType;
use crate::{
    types::{
        feeds::{Feed, FeedError, FeedStorage, Source},
        whitelist::{Whitelist, WhitelistError},
    },
    utils::{
        siwe::{self, SiweError},
        validation,
    },
};

#[derive(Error, Debug)]
pub enum CustomFeedError {
    #[error("SIWE Error: {0}")]
    SIWEError(#[from] SiweError),
    #[error("Validation Error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Whitelist Error: {0}")]
    WhitelistError(#[from] WhitelistError),
    #[error("Feed Error: {0}")]
    FeedError(#[from] FeedError),
    #[error("Feed already exists")]
    FeedAlreadyExists,
    #[error("Feed not found")]
    FeedNotFound,
    #[error("Not feed owner")]
    NotFeedOwner,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateCustomFeedRequest {
    pub id: String,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
    pub feed_type: FeedType,
    pub decimals: Option<u64>,
    #[validate(length(min = 1, max = 5))]
    // second one is used for a nested validation of all sources
    #[validate]
    pub sources: Vec<Source>,
    pub msg: String,
    pub sig: String,
}

#[update]
pub async fn create_custom_feed(req: CreateCustomFeedRequest) -> Result<(), String> {
    _create_custom_feed(req)
        .await
        .map_err(|e| format!("Failed to a create custom feed: {e}"))
}

pub async fn _create_custom_feed(mut req: CreateCustomFeedRequest) -> Result<(), CustomFeedError> {
    req.id = format!("custom_{}", req.id);

    let addr = siwe::recover(&req.msg, &req.sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    if FeedStorage::contains(&req.id) {
        return Err(CustomFeedError::FeedAlreadyExists)?;
    }

    req.validate()?;

    let mut feed = Feed::from(req.clone());
    feed.set_owner(addr.clone());

    FeedStorage::get_custom_rate(&feed, &req.sources).await?;
    FeedStorage::add(feed);

    metrics!(inc CUSTOM_FEEDS);

    log!(
        "[FEEDS] custom feed created. id: {}, owner: {}",
        req.id,
        addr
    );
    Ok(())
}

#[update]
pub async fn remove_custom_feed(id: String, msg: String, sig: String) -> Result<(), String> {
    _remove_custom_feed(id, msg, sig)
        .await
        .map_err(|e| format!("Failed to remove custom feed: {e}"))
}

#[inline(always)]
pub async fn _remove_custom_feed(
    id: String,
    msg: String,
    sig: String,
) -> Result<(), CustomFeedError> {
    let addr = siwe::recover(&msg, &sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    if let Some(feed) = FeedStorage::get(&id) {
        if feed.owner != addr {
            return Err(CustomFeedError::NotFeedOwner)?;
        }

        FeedStorage::remove(&id);

        metrics!(dec CUSTOM_FEEDS);
        log!("[FEEDS] custom feed removed. id: {}, owner: {}", id, addr);
        return Ok(());
    }

    Err(CustomFeedError::FeedNotFound)
}
