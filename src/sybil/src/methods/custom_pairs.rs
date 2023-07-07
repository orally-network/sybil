use ic_cdk::{query, update};

use ic_cdk::export::{
    candid::{CandidType, Nat},
    serde::{Deserialize, Serialize},
};

use anyhow::Result;
use thiserror::Error;
use validator::Validate;

use crate::{types::{custom_pair::CustomPair, pairs::Source}, utils::{siwe, validation}, STATE};

#[derive(Error, Debug)]
pub enum CustomPairError {
    #[error("SIWE Error")]
    SIWEError(#[from] siwe::SIWEError),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct CreateCustomPairRequest {
    #[validate(regex = "validation::PAIR_ID_REGEX")]
    pub pair_id: String,
    #[validate(custom = "validation::validate_update_freq")]
    pub update_freq: Nat,
    pub decimals: Nat,
    pub executions: Nat,
    #[validate(length(min = 1))]
    // second one is used for a nested validation of all sources
    #[validate]
    pub endpoints: Vec<Source>,
    pub msg: String,
    pub sig: String,
}

#[update]
pub async fn create_custom_pair(req: CreateCustomPairRequest) -> Result<(), String> {
    _create_custom_pair(req).await.map_err(|e| e.to_string())
}

pub async fn _create_custom_pair(req: CreateCustomPairRequest) -> Result<(), CustomPairError> {
    let _addr = siwe::recover(&req.msg, &req.sig).await?;    

    // let custom_pair = CustomPairBuilder::new(&req.pair_id)?
    //     .frequency(nat_to_u64(req.frequency))?
    //     .source(&req.uri, &req.resolver)
    //     .await?
    //     .estimate_cost(hex::encode(addr.as_bytes()), req.amount)
    //     .await?
    //     .build()?;

    // STATE.with(|state| {
    //     let mut state = state.borrow_mut();
    //     state.custom_pairs.push(custom_pair.clone());
    // });

    // log_message(format!("Custom pair created, pair id: {}", custom_pair.id));

    Ok(())
}

#[update]
pub fn remove_custom_pair(pair_id: String) {
    STATE.with(|state| {
        let custom_pairs = &mut state.borrow_mut().custom_pairs;
        if let Some(index) = custom_pairs.iter().position(|pair| pair.id == pair_id) {
            custom_pairs.remove(index);
        }
    });
}

#[query]
pub fn get_custom_pairs() -> Vec<CustomPair> {
    STATE.with(|state| state.borrow().custom_pairs.clone())
}
