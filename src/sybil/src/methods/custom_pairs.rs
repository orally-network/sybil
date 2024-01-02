use candid::{CandidType, Nat};
use ic_cdk::update;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::log;
use crate::metrics;
use crate::{
    types::{
        pairs::{Pair, PairError, PairsStorage, Source},
        whitelist::{Whitelist, WhitelistError},
    },
    utils::{
        siwe::{self, SiweError},
        validation,
    },
};

#[derive(Error, Debug)]
pub enum CustomPairError {
    #[error("SIWE Error: {0}")]
    SIWEError(#[from] SiweError),
    #[error("Validation Error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Whitelist Error: {0}")]
    WhitelistError(#[from] WhitelistError),
    #[error("Pair Error: {0}")]
    PairError(#[from] PairError),
    #[error("Pair already exists")]
    PairAlreadyExists,
    #[error("Pair not found")]
    PairNotFound,
    #[error("Not pair owner")]
    NotPairOwner,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateCustomPairRequest {
    #[validate(regex = "validation::PAIR_ID_REGEX")]
    pub pair_id: String,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
    pub decimals: Nat,
    #[validate(length(min = 1, max = 5))]
    // second one is used for a nested validation of all sources
    #[validate]
    pub sources: Vec<Source>,
    pub msg: String,
    pub sig: String,
}

#[update]
pub async fn create_custom_pair(req: CreateCustomPairRequest) -> Result<(), String> {
    _create_custom_pair(req)
        .await
        .map_err(|e| format!("Failed to a create custom pair: {e}"))
}

pub async fn _create_custom_pair(req: CreateCustomPairRequest) -> Result<(), CustomPairError> {
    let addr = siwe::recover(&req.msg, &req.sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    if PairsStorage::contains(&req.pair_id) {
        return Err(CustomPairError::PairAlreadyExists)?;
    }

    req.validate()?;

    let mut pair = Pair::from(req.clone());
    pair.set_owner(addr.clone());

    PairsStorage::get_custom_rate(&pair, &req.sources).await?;
    PairsStorage::add(pair);

    metrics!(inc CUSTOM_PAIRS);

    log!(
        "[PAIRS] custom pair created. id: {}, owner: {}",
        req.pair_id,
        addr
    );
    Ok(())
}

#[update]
pub async fn remove_custom_pair(id: String, msg: String, sig: String) -> Result<(), String> {
    _remove_custom_pair(id, msg, sig)
        .await
        .map_err(|e| format!("Failed to remove custom pair: {e}"))
}

#[inline(always)]
pub async fn _remove_custom_pair(
    id: String,
    msg: String,
    sig: String,
) -> Result<(), CustomPairError> {
    let addr = siwe::recover(&msg, &sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    if let Some(pair) = PairsStorage::get(&id) {
        if pair.owner != addr {
            return Err(CustomPairError::NotPairOwner)?;
        }

        PairsStorage::remove(&id);

        metrics!(dec CUSTOM_PAIRS);
        log!("[PAIRS] custom pair removed. id: {}, owner: {}", id, addr);
        return Ok(());
    }

    Err(CustomPairError::PairNotFound)
}
