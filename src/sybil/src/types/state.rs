use std::str::FromStr;

use candid::Principal;
use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::{
    config::{Cfg, UpdateCfg},
    data_fetchers::{DataFetchersStorage, DataFethcersIndexer},
    pairs::PairsStorage,
    whitelist::Whitelist,
    Address,
};
use crate::{
    types::balances::{Balances, BalancesCfg},
    STATE,
};

#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct State {
    pub exchange_rate_canister: Principal,
    pub key_name: String,
    pub mock: bool,
    pub pairs: PairsStorage,
    pub balances: Balances,
    pub balances_cfg: BalancesCfg,
    pub eth_address: Option<Address>,
    pub whitelist: Whitelist,
    pub data_fetchers: DataFetchersStorage,
    pub data_fetchers_indexer: DataFethcersIndexer,
}

impl Default for State {
    fn default() -> Self {
        Self {
            exchange_rate_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            key_name: "".to_string(),
            mock: false,
            pairs: PairsStorage::default(),
            balances: Balances::default(),
            balances_cfg: BalancesCfg::default(),
            eth_address: None,
            whitelist: Whitelist::default(),
            data_fetchers: DataFetchersStorage::default(),
            data_fetchers_indexer: DataFethcersIndexer::default(),
        }
    }
}

pub fn init(cfg: &Cfg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.exchange_rate_canister = cfg.exchange_rate_canister;
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
            mock: state.mock,
            key_name: state.key_name.clone(),
            balances_cfg: state.balances_cfg.clone(),
        }
    })
}

pub fn clear() {
    PairsStorage::clear();
    Balances::clear();
    Whitelist::clear();
    DataFetchersStorage::clear();
    DataFethcersIndexer::reset();
}
