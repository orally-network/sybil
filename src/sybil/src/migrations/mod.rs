use candid::{CandidType, Principal};
use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::monitor;
use serde::{Deserialize, Serialize};

use crate::{
    http::HttpService,
    log, metrics,
    types::{
        balances::{Balances, BalancesCfg},
        cache::{HttpCache, RateCache, SignaturesCache},
        data_fetchers::{DataFetchersStorage, DataFethcersIndexer},
        pairs::{PairType, PairsStorage},
        state::State,
        whitelist::Whitelist,
        Address,
    },
    utils::{
        canister::set_custom_panic_hook,
        metrics::{Metric, Metrics, METRICS},
    },
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

#[allow(non_snake_case)]
#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct OldMetrics {
    pub CUSTOM_PAIRS: Option<Metric>,
    pub DEFAULT_PAIRS: Option<Metric>,
    pub GET_ASSET_DATA_CALLS: Option<Metric>,
    pub SUCCESSFUL_GET_ASSET_DATA_CALLS: Option<Metric>,
    pub GET_ASSET_DATA_WITH_PROOF_CALLS: Option<Metric>,
    pub SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS: Option<Metric>,
    pub FALLBACK_XRC_CALLS: Option<Metric>,
    pub SUCCESSFUL_FALLBACK_XRC_CALLS: Option<Metric>,
    pub XRC_CALLS: Option<Metric>,
    pub SUCCESSFUL_XRC_CALLS: Option<Metric>,
    pub CYCLES: Option<Metric>,
}

impl From<OldMetrics> for Metrics {
    fn from(value: OldMetrics) -> Self {
        Metrics {
            CUSTOM_PAIRS: value.CUSTOM_PAIRS.unwrap_or_default(),
            DEFAULT_PAIRS: value.DEFAULT_PAIRS.unwrap_or_default(),
            GET_ASSET_DATA_CALLS: value.GET_ASSET_DATA_CALLS.unwrap_or_default(),
            SUCCESSFUL_GET_ASSET_DATA_CALLS: value
                .SUCCESSFUL_GET_ASSET_DATA_CALLS
                .unwrap_or_default(),
            GET_ASSET_DATA_WITH_PROOF_CALLS: value
                .GET_ASSET_DATA_WITH_PROOF_CALLS
                .unwrap_or_default(),
            SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS: value
                .SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS
                .unwrap_or_default(),
            FALLBACK_XRC_CALLS: value.FALLBACK_XRC_CALLS.unwrap_or_default(),
            SUCCESSFUL_FALLBACK_XRC_CALLS: value.SUCCESSFUL_FALLBACK_XRC_CALLS.unwrap_or_default(),
            XRC_CALLS: value.XRC_CALLS.unwrap_or_default(),
            SUCCESSFUL_XRC_CALLS: value.SUCCESSFUL_XRC_CALLS.unwrap_or_default(),
            CYCLES: value.CYCLES.unwrap_or_default(),
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

    let metrics = METRICS.with(|metrics| metrics.take());

    storage::stable_save((
        state,
        cache,
        monitor_data,
        http_cache,
        signatures_cache,
        metrics,
    ))
    .expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, cache, monitor_data, http_cache, signatures_cache, metrics): (
        OldState,
        RateCache,
        monitor::PostUpgradeStableData,
        HttpCache,
        SignaturesCache,
        Option<OldMetrics>,
    ) = storage::stable_restore().expect("should be able to restore");

    monitor::post_upgrade_stable_data(monitor_data);

    let state = State::from(state);

    set_custom_panic_hook();

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache));
    HTTP_CACHE.with(|c| c.replace(http_cache));
    SIGNATURES_CACHE.with(|c| c.replace(signatures_cache));

    if let Some(metrics) = metrics {
        METRICS.with(|m| m.replace(metrics.into()));

        STATE.with(|state| {
            let state = state.borrow();
            let pairs = &state.pairs;
            let mut default_pairs = 0;
            let mut custom_pairs = 0;
            for (_, pair) in pairs.0.iter() {
                match pair.pair_type {
                    PairType::Default => {
                        default_pairs += 1;
                    }
                    PairType::Custom { .. } => {
                        custom_pairs += 1;
                    }
                }
            }

            metrics!(set DEFAULT_PAIRS, default_pairs);
            metrics!(set CUSTOM_PAIRS, custom_pairs);
        });
    }

    log!("Post upgrade finished");

    HttpService::init();
}
