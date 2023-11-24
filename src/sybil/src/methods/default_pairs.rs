use candid::{CandidType, Nat};
use ic_cdk::update;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

use crate::{
    log,
    types::{
        pairs::{Pair, PairError, PairsStorage},
        whitelist::WhitelistError,
    },
    utils::{validate_caller, validation, CallerError},
};

#[derive(Error, Debug)]
pub enum DefaultPairError {
    #[error("Whitelist error: {0}")]
    WhitelistError(#[from] WhitelistError),
    #[error("Pair error: {0}")]
    PairError(#[from] PairError),
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
    #[error("Pair already exists")]
    PairAlreadyExists,
    #[error("Pair not found")]
    PairNotFound,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateDefaultPairRequest {
    #[validate(regex = "validation::PAIR_ID_REGEX")]
    pub pair_id: String,
    pub decimals: Nat,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
}

#[update]
pub async fn create_default_pair(req: CreateDefaultPairRequest) -> Result<(), String> {
    _create_default_pair(req)
        .await
        .map_err(|err| format!("failed to add a pair: {err}"))
}

async fn _create_default_pair(req: CreateDefaultPairRequest) -> Result<(), DefaultPairError> {
    validate_caller()?;
    if PairsStorage::contains(&req.pair_id) {
        return Err(DefaultPairError::PairAlreadyExists);
    }

    let pair = Pair::from(req.clone());

    PairsStorage::get_default_rate(&pair).await?;
    PairsStorage::add(pair);

    log!("[PAIRS] default pair added. Pair ID: {}", req.pair_id);
    Ok(())
}

#[update]
pub async fn remove_default_pair(pair_id: String) -> Result<(), String> {
    _remove_default_pair(pair_id)
        .await
        .map_err(|err| format!("failed to remove a pair: {err}"))
}

async fn _remove_default_pair(id: String) -> Result<(), DefaultPairError> {
    validate_caller()?;
    if !PairsStorage::contains(&id) {
        return Err(DefaultPairError::PairNotFound);
    }

    PairsStorage::remove(&id);

    log!("[PAIRS] default pair removed. Pair ID: {}", id);
    Ok(())
}
