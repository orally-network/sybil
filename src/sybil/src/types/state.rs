use std::{str::FromStr, collections::HashMap};

use candid::{Nat, Principal};
use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::{
    config::{Cfg, UpdateCfg},
    custom_pair::CustomPair,
    pairs::Pair,
    rate_data::RateDataLight,
};
use crate::{utils::nat, STATE, types::balances::{Balances, BalancesCfg}};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldPair {
    pub id: String,
    pub frequency: u64,
    pub last_update: u64,
    pub data: RateDataLight,
}

#[derive(Clone, Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub exchange_rate_canister: String,
    pub siwe_signer_canister: String,
    pub old_pairs: Vec<OldPair>,
    pub custom_pairs: Vec<CustomPair>,
    pub key_name: String,
    pub cache_expiration: u64,
    pub treasurer_canister: String,
    pub cost_per_execution: u64,
    pub pairs: HashMap<String, Pair>,
    pub balances: Balances,
    pub balances_cfg: BalancesCfg,
}

pub fn init(cfg: &Cfg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.exchange_rate_canister = cfg.exchange_rate_canister.to_string();
        state.siwe_signer_canister = cfg.siwe_signer_canister.to_string();
        state.treasurer_canister = cfg.treasurer_canister.to_string();
        state.key_name = cfg.key_name.clone();
        state.cache_expiration = nat::to_u64(&cfg.cache_expiration);
        state.cost_per_execution = nat::to_u64(&cfg.cost_per_execution);
        state.balances_cfg = cfg.balances_cfg.clone();
    });
}

pub fn update(cfg: &UpdateCfg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(exchange_rate_canister) = &cfg.exchange_rate_canister {
            state.exchange_rate_canister = exchange_rate_canister.to_string();
        }
        if let Some(siwe_signer_canister) = &cfg.siwe_signer_canister {
            state.siwe_signer_canister = siwe_signer_canister.to_string();
        }
        if let Some(treasurer_canister) = &cfg.treasurer_canister {
            state.treasurer_canister = treasurer_canister.to_string();
        }
        if let Some(key_name) = &cfg.key_name {
            state.key_name = key_name.clone();
        }
        if let Some(cache_expiration) = &cfg.cache_expiration {
            state.cache_expiration = nat::to_u64(cache_expiration);
        }
        if let Some(cost_per_execution) = &cfg.cost_per_execution {
            state.cost_per_execution = nat::to_u64(cost_per_execution);
        }
        if let Some(balances_cfg) = &cfg.balances_cfg {
            state.balances_cfg = balances_cfg.clone();
        }
    });
}

pub fn get_cfg() -> Cfg {
    STATE.with(|state| {
        let state = state.borrow();
        Cfg {
            exchange_rate_canister: Principal::from_str(&state.exchange_rate_canister)
                .expect("Invalid principal"),
            siwe_signer_canister: Principal::from_str(&state.siwe_signer_canister)
                .expect("Invalid principal"),
            treasurer_canister: Principal::from_str(&state.treasurer_canister)
                .expect("Invalid principal"),
            key_name: state.key_name.clone(),
            cache_expiration: Nat::from(state.cache_expiration),
            cost_per_execution: Nat::from(state.cost_per_execution),
            balances_cfg: state.balances_cfg.clone(),
        }
    })
}
