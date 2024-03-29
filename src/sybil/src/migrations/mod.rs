#![allow(deprecated)]

use std::collections::HashMap;

use candid::{CandidType, Nat, Principal};
use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::{logger, monitor};
use serde::{Deserialize, Serialize};

use crate::{
    http::HttpService,
    log, metrics,
    types::{
        balances::{Balances, BalancesCfg},
        cache::{HttpCache, RateCache, SignaturesCache},
        feeds::{Feed, FeedStatus, FeedStorage, FeedType},
        rate_data::AssetDataResult,
        source::{HttpSource, Source},
        state::State,
        whitelist::Whitelist,
        Address, Seconds, Timestamp,
    },
    utils::{
        canister::set_custom_panic_hook,
        metrics::{Metric, Metrics, METRICS},
    },
    CACHE, HTTP_CACHE, SIGNATURES_CACHE, STATE,
};

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct OldRateCache(HashMap<String, OldRateCacheEntry>);

impl From<OldRateCache> for RateCache {
    fn from(_: OldRateCache) -> Self {
        RateCache::default()
    }
}

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
struct OldRateCacheEntry {
    expired_at: u64,
    data: Option<AssetDataResult>,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldFeedStorage(HashMap<String, OldFeed>);

impl From<OldFeedStorage> for FeedStorage {
    fn from(old: OldFeedStorage) -> Self {
        let new = old
            .0
            .into_iter()
            .map(|(id, feed)| (id, feed.into()))
            .collect();

        FeedStorage(new)
    }
}

impl From<OldFeed> for Feed {
    fn from(old: OldFeed) -> Self {
        Self {
            id: old.id,
            feed_type: old.feed_type,
            update_freq: old.update_freq,
            sources: old.sources.clone(),
            new_sources: if let Some(sources) = old.sources {
                Some(
                    sources
                        .into_iter()
                        .map(|s| {
                            Source::HttpSource(HttpSource {
                                uri: s.uri,
                                api_keys: s.api_keys,
                                resolver: s.resolver,
                                expected_bytes: s.expected_bytes,
                            })
                        })
                        .collect(),
                )
            } else {
                old.new_sources
            },
            decimals: old.decimals,
            status: old.status.into(),
            owner: old.owner,
            data: old.data,
        }
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldFeed {
    pub id: String,
    pub feed_type: FeedType,
    pub update_freq: Seconds,
    pub sources: Option<Vec<HttpSource>>,
    pub new_sources: Option<Vec<Source>>,
    pub decimals: Option<u64>,
    pub status: FeedStatus,
    pub owner: Address,
    pub data: Option<AssetDataResult>,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldFeedStatus {
    last_update: Timestamp,
    updated_counter: u64,
    requests_counter: u64,
}

impl From<OldFeedStatus> for FeedStatus {
    fn from(old: OldFeedStatus) -> Self {
        Self {
            last_update: old.last_update,
            updated_counter: old.updated_counter,
            requests_counter: old.requests_counter,
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFetchersStorage(HashMap<Nat, DataFetcher>);

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFethcersIndexer(Nat);

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFetcher {
    pub id: Nat,
    pub update_freq: Nat,
    pub owner: Address,
    pub sources: Vec<Source>,
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct OldState {
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Option<Principal>,
    pub evm_rpc_canister: Option<Principal>,
    pub rpc_wrapper: Option<String>,
    pub key_name: String,
    pub mock: bool,
    pub feeds: OldFeedStorage,
    pub balances: Balances,
    pub balances_cfg: BalancesCfg,
    pub eth_address: Option<Address>,
    pub whitelist: Whitelist,
    pub data_fetchers: Option<DataFetchersStorage>,
    pub data_fetchers_indexer: Option<DataFethcersIndexer>,
}

impl From<OldState> for State {
    fn from(state: OldState) -> Self {
        Self {
            exchange_rate_canister: state.exchange_rate_canister,
            fallback_xrc: state.fallback_xrc.unwrap_or_else(|| {
                Principal::from_text("a3uxy-eiaaa-aaaao-a2qaa-cai").expect("Invalid principal")
            }),
            evm_rpc_canister: state.evm_rpc_canister.unwrap_or_else(|| {
                Principal::from_text("aovwi-4maaa-aaaaa-qaagq-cai").expect("Invalid principal")
            }),
            rpc_wrapper: state.rpc_wrapper.unwrap_or_default(),
            key_name: state.key_name,
            mock: state.mock,
            feeds: state.feeds.into(),
            balances: state.balances,
            balances_cfg: state.balances_cfg,
            eth_address: state.eth_address,
            whitelist: state.whitelist,
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
            CUSTOM_FEEDS: value.CUSTOM_PAIRS.unwrap_or_default(),
            DEFAULT_FEEDS: value.DEFAULT_PAIRS.unwrap_or_default(),
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

    let log_data = logger::pre_upgrade_stable_data();
    let monitor_data = monitor::pre_upgrade_stable_data();

    let metrics = METRICS.with(|metrics| metrics.take());

    storage::stable_save((
        state,
        cache,
        log_data,
        monitor_data,
        http_cache,
        signatures_cache,
        metrics,
    ))
    .expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, cache, log_data, monitor_data, http_cache, signatures_cache, metrics): (
        OldState,
        OldRateCache,
        logger::PostUpgradeStableData,
        monitor::PostUpgradeStableData,
        HttpCache,
        SignaturesCache,
        Option<OldMetrics>,
    ) = storage::stable_restore().expect("should be able to restore");

    logger::post_upgrade_stable_data(log_data);
    monitor::post_upgrade_stable_data(monitor_data);

    let state = State::from(state);

    set_custom_panic_hook();

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache.into()));
    HTTP_CACHE.with(|c| c.replace(http_cache));
    SIGNATURES_CACHE.with(|c| c.replace(signatures_cache));

    if let Some(metrics) = metrics {
        METRICS.with(|m| m.replace(metrics.into()));

        STATE.with(|state| {
            let state = state.borrow();
            let feeds = &state.feeds;
            let mut default_feeds = 0;
            let mut custom_feeds = 0;
            for (_, feed) in feeds.0.iter() {
                match feed.feed_type {
                    FeedType::Default => {
                        default_feeds += 1;
                    }
                    FeedType::Custom => {
                        custom_feeds += 1;
                    }
                    _ => {}
                }
            }

            metrics!(set DEFAULT_FEEDS, default_feeds);
            metrics!(set CUSTOM_FEEDS, custom_feeds);
        });
    }

    log!("Post upgrade finished");

    HttpService::init();
}
