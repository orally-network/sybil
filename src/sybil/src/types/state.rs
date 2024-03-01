use std::str::FromStr;

use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

use super::{
    config::{Cfg, UpdateCfg},
    feeds::FeedStorage,
    whitelist::Whitelist,
    Address,
};
use crate::{
    types::balances::{Balances, BalancesCfg},
    STATE,
};

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct State {
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Principal,
    pub evm_rpc_canister: Principal,
    pub rpc_wrapper: String,
    pub key_name: String,
    pub mock: bool,
    pub feeds: FeedStorage,
    pub balances: Balances,
    pub balances_cfg: BalancesCfg,
    pub eth_address: Option<Address>,
    pub whitelist: Whitelist,
}

impl Default for State {
    fn default() -> Self {
        Self {
            exchange_rate_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            fallback_xrc: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            evm_rpc_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            rpc_wrapper: "".to_string(),
            key_name: "".to_string(),
            mock: false,
            feeds: FeedStorage::default(),
            balances: Balances::default(),
            balances_cfg: BalancesCfg::default(),
            eth_address: None,
            whitelist: Whitelist::default(),
        }
    }
}

pub fn init(cfg: &Cfg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.exchange_rate_canister = cfg.exchange_rate_canister;
        state.fallback_xrc = cfg.fallback_xrc;
        state.evm_rpc_canister = cfg.evm_rpc_canister;
        state.rpc_wrapper = cfg.rpc_wrapper.clone();
        state.key_name = cfg.key_name.clone();
        state.balances_cfg = cfg.balances_cfg.clone();
        state.mock = cfg.mock;
    });
}

pub fn update(cfg: &UpdateCfg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(exchange_rate_canister) = &cfg.exchange_rate_canister {
            state.exchange_rate_canister = *exchange_rate_canister;
        }
        if let Some(fallback_xrc) = &cfg.fallback_xrc {
            state.fallback_xrc = *fallback_xrc;
        }
        if let Some(evm_rpc_canister) = &cfg.evm_rpc_canister {
            state.evm_rpc_canister = *evm_rpc_canister;
        }
        if let Some(rpc_wrapper) = &cfg.rpc_wrapper {
            state.rpc_wrapper = rpc_wrapper.clone();
        }
        if let Some(mock) = &cfg.mock {
            state.mock = *mock;
        }
        if let Some(key_name) = &cfg.key_name {
            state.key_name = key_name.clone();
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
            exchange_rate_canister: state.exchange_rate_canister,
            fallback_xrc: state.fallback_xrc,
            evm_rpc_canister: state.evm_rpc_canister,
            rpc_wrapper: state.rpc_wrapper.clone(),
            mock: state.mock,
            key_name: state.key_name.clone(),
            balances_cfg: state.balances_cfg.clone(),
        }
    })
}

pub fn clear() {
    FeedStorage::clear();
    Balances::clear();
    Whitelist::clear();
}
