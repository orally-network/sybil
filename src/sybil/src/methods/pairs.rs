use std::time::Duration;

use anyhow::{anyhow, Result};

use ic_cdk::{export::{candid::{Nat, CandidType}, serde::{Serialize, Deserialize}}, query, update};
use ic_utils::logger::log_message;

use crate::{
    types::state::OldPair,
    utils::{get_rate::get_rate_with_cache, is_valid_pair_id, nat_to_u64, validate_caller},
    STATE,
};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CreatePairRequest {
    pub pair_id: String,
    pub decimals: Nat,
    pub update_freq: Nat,
}

#[update]
pub async fn add_pair(pair_id: String, frequency: Nat) -> Result<OldPair, String> {
    _add_pair(pair_id, frequency)
        .await
        .map_err(|err| err.to_string())
}

pub async fn _add_pair(pair_id: String, frequency: Nat) -> Result<OldPair> {
    validate_caller()?;

    if !is_valid_pair_id(&pair_id) {
        return Err(anyhow!("Pair ID is invalid"));
    }

    let data = get_rate_with_cache(&pair_id, false).await?;

    let pair = OldPair {
        id: pair_id,
        last_update: Duration::from_nanos(ic_cdk::api::time()).as_secs(),
        frequency: nat_to_u64(frequency),
        data,
    };

    STATE.with(|state| {
        state.borrow_mut().old_pairs.push(pair.clone());
    });

    log_message(format!("Pair created, pair id: {}", pair.id));

    Ok(pair)
}

#[update]
pub fn remove_pair(pair_id: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| {
        let pairs = &mut state.borrow_mut().old_pairs;
        if let Some(index) = pairs.iter().position(|pair| pair.id == pair_id) {
            pairs.remove(index);
        }
    });
}

#[query]
pub fn get_pairs() -> Vec<OldPair> {
    STATE.with(|state| state.borrow().old_pairs.clone())
}
