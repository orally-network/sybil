use candid::{CandidType, Principal};
use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::monitor;
use serde::{Deserialize, Serialize};

use crate::{
    http::HttpService,
    log,
    types::{
        balances::{Balances, BalancesCfg},
        cache::{HttpCache, RateCache, SignaturesCache},
        data_fetchers::{DataFetchersStorage, DataFethcersIndexer},
        pairs::PairsStorage,
        state::State,
        whitelist::Whitelist,
        Address,
    },
    utils::canister::set_custom_panic_hook,
    CACHE, HTTP_CACHE, SIGNATURES_CACHE, STATE,
};

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct OldState {
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Option<Principal>,
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

impl From<State> for OldState {
    fn from(state: State) -> Self {
        Self {
            exchange_rate_canister: state.exchange_rate_canister,
            fallback_xrc: Some(state.fallback_xrc),
            key_name: state.key_name,
            mock: state.mock,
            pairs: state.pairs,
            balances: state.balances,
            balances_cfg: state.balances_cfg,
            eth_address: state.eth_address,
            whitelist: state.whitelist,
            data_fetchers: state.data_fetchers,
            data_fetchers_indexer: state.data_fetchers_indexer,
        }
    }
}

impl From<OldState> for State {
    fn from(state: OldState) -> Self {
        Self {
            exchange_rate_canister: state.exchange_rate_canister,
            fallback_xrc: state.fallback_xrc.unwrap_or_else(|| {
                Principal::from_text("a3uxy-eiaaa-aaaao-a2qaa-cai").expect("Invalid principal")
            }),
            key_name: state.key_name,
            mock: state.mock,
            pairs: state.pairs,
            balances: state.balances,
            balances_cfg: state.balances_cfg,
            eth_address: state.eth_address,
            whitelist: state.whitelist,
            data_fetchers: state.data_fetchers,
            data_fetchers_indexer: state.data_fetchers_indexer,
        }
    }
}

#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|state| state.borrow().clone());
    let cache = CACHE.with(|cache| cache.borrow().clone());
    let http_cache = HTTP_CACHE.with(|http_cache| http_cache.borrow().clone());
    let signatures_cache =
        SIGNATURES_CACHE.with(|signatures_cache| signatures_cache.borrow().clone());

    let monitor_data = monitor::pre_upgrade_stable_data();

    storage::stable_save((state, cache, monitor_data, http_cache, signatures_cache))
        .expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, cache, monitor_data, http_cache, signatures_cache): (
        OldState,
        RateCache,
        monitor::PostUpgradeStableData,
        HttpCache,
        SignaturesCache,
    ) = storage::stable_restore().expect("should be able to restore");

    monitor::post_upgrade_stable_data(monitor_data);

    let state = State::from(state);

    set_custom_panic_hook();

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache));
    HTTP_CACHE.with(|c| c.replace(http_cache));
    SIGNATURES_CACHE.with(|c| c.replace(signatures_cache));

    log!("Post upgrade finished");

    HttpService::init();
}
