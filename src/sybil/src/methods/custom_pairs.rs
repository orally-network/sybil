use ic_cdk::{query, update};

use ic_cdk::export::{
    candid::{CandidType, Nat},
    serde::{Deserialize, Serialize},
};

use anyhow::{Context, Result};

use crate::{
    types::{
        custom_pair::{CustomPair, CustomPairBuilder},
        rate_data::CustomPairData,
    },
    utils::{nat_to_u64, rec_eth_addr},
    STATE,
};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CreateCustomPairRequest {
    pub pair_id: String,
    pub frequency: Nat,
    pub uri: String,
    pub msg: String,
    pub sig: String,
}

#[update]
pub async fn create_custom_pair(req: CreateCustomPairRequest) -> Result<(), String> {
    _create_custom_pair(req).await.map_err(|e| e.to_string())
}

pub async fn _create_custom_pair(req: CreateCustomPairRequest) -> Result<()> {
    let pub_key = rec_eth_addr(&req.msg, &req.sig).await?;

    let custom_pair = CustomPairBuilder::new(&req.pair_id)?
        .frequency(nat_to_u64(req.frequency))?
        .source(&req.uri, &pub_key)
        .await?
        .build()?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.custom_pairs.push(custom_pair);
    });

    Ok(())
}

#[query]
pub fn get_asset_data_with_proof(pair_id: String) -> Result<CustomPairData, String> {
    _get_asset_data_with_proof(pair_id).map_err(|e| e.to_string())
}

pub fn _get_asset_data_with_proof(pair_id: String) -> Result<CustomPairData> {
    STATE.with(|state| {
        let state = state.borrow();

        state
            .custom_pairs
            .iter()
            .find(|pair| pair.id == pair_id)
            .context("pair not found")
            .map(|v| v.data.clone())
    })
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
