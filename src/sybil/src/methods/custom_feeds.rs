use candid::{CandidType, Nat};
use ic_cdk::update;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::log;
use crate::metrics;
use crate::types::balances::BalanceError;
use crate::types::cache::SignaturesCacheError;
use crate::types::chains_rpc::ChainsRPCError;
use crate::types::feeds::FeedType;
use crate::types::source::Source;
use crate::utils::address::AddressError;
use crate::utils::canister::CanisterError;
use crate::utils::encoding::ParseTokensError;
use crate::utils::web3;
use crate::{
    types::{
        feeds::{Feed, FeedError, FeedStorage},
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
    #[error("Balance Error: {0}")]
    BalanceError(#[from] BalanceError),
    #[error("Signatures error: {0}")]
    SignaturesError(#[from] SignaturesCacheError),
    #[error("Chains RPC error: {0}")]
    ChainsRPCError(#[from] ChainsRPCError),
    #[error("address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("Web3 Error: {0}")]
    Web3Error(#[from] web3::Web3Error),
    #[error("Parse Tokens Error")]
    ParseTokensError(#[from] ParseTokensError),
    #[error("Canister Error: {0}")]
    CanisterError(#[from] CanisterError),
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
    #[error("Failed to parse ABI: {0}")]
    FailedToParseABI(String),
    #[error("Abi doesn't contain method: {0}")]
    AbiDoesntContainMethod(String),
    #[error("Cache error: {0}")]
    CacheError(#[from] crate::types::cache::CacheError),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateCustomFeedRequest {
    pub id: String,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
    pub feed_type: FeedType,
    pub decimals: Option<u64>,
    #[validate(length(min = 1, max = 5))]
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

    FeedStorage::get_custom_rate(&feed, &req.sources, Some(addr.clone())).await?;
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
