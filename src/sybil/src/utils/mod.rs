pub mod encoding;
pub mod exchange_rate;
pub mod get_rate;
pub mod macros;
pub mod nat;
pub mod signature;
pub mod time;
pub mod treasurer;
pub mod siwe;
pub mod convertion;
pub mod address;
pub mod validation;
pub mod web3;
pub mod processors;

use std::str::FromStr;

use ic_cdk::export::{candid::Nat, Principal};
use ic_web3_rs::types::H160;

use anyhow::{anyhow, Context, Result};

use crate::{types::PairType, STATE};

#[derive(Clone, Debug)]
pub struct PairMetadata {
    pub pair_id: String,
    pub pair_type: PairType,
    pub index: usize,
}

pub fn nat_to_u64(nat: Nat) -> u64 {
    *nat.0
        .to_u64_digits()
        .last()
        .expect("should be at least one digit")
}

pub fn is_pair_exist(pair_id: &str) -> (bool, Option<PairMetadata>) {
    STATE.with(|state| {
        let state = state.borrow();

        let index = state.old_pairs.iter().position(|p| p.id == pair_id);
        if let Some(index) = index {
            return (
                true,
                Some(PairMetadata {
                    pair_id: pair_id.into(),
                    pair_type: PairType::Pair,
                    index,
                }),
            );
        }

        let index = state.custom_pairs.iter().position(|p| p.id == pair_id);
        if let Some(index) = index {
            return (
                true,
                Some(PairMetadata {
                    pair_id: pair_id.into(),
                    pair_type: PairType::CustomPair,
                    index,
                }),
            );
        }

        (false, None)
    })
}

pub fn is_valid_pair_id(pair_id: &str) -> bool {
    let artifact: Vec<&str> = pair_id.split_terminator('/').collect();

    if artifact.len() != 2 {
        return false;
    }

    true
}

#[allow(dead_code)]
pub async fn rec_eth_addr(msg: &str, sig: &str) -> Result<H160> {
    let siwe_canister = STATE.with(|state| state.borrow().siwe_signer_canister.clone());

    let siwe_canister = Principal::from_text(siwe_canister).expect("canister should be valid");

    let msg = msg.to_string();
    let sig = sig.to_string();

    let (signer,): (String,) = ic_cdk::call(siwe_canister, "get_signer", (msg, sig))
        .await
        .map_err(|(code, msg)| anyhow!("{:?}: {}", code, msg))?;

    H160::from_str(&signer).context("failed to parse signer address")
}

pub fn validate_caller() -> Result<()> {
    if ic_cdk::api::is_controller(&ic_cdk::caller()) {
        return Ok(());
    }

    Err(anyhow!("caller is not a conroller"))
}
