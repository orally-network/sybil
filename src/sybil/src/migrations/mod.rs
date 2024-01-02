use std::collections::HashMap;

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
        feeds::{Source, FeedStorage, Feed, FeedType, FeedStatus},
        state::State,
        whitelist::Whitelist,
        Address, rate_data::RateDataLight, Seconds, Timestamp,
    },
    utils::canister::set_custom_panic_hook,
    CACHE, HTTP_CACHE, SIGNATURES_CACHE, STATE,
};


#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldFeedStorage(HashMap<String, OldFeed>);

impl From<OldFeedStorage> for FeedStorage {
    fn from(old: OldFeedStorage) -> Self {
        let new = old.0.into_iter().map(|(id, feed)| (id, feed.into())).collect();

        FeedStorage(new)
    }
}


impl From<OldFeed> for Feed {
    fn from(old: OldFeed) -> Self {
        Self {
            id: old.id,
            feed_type: old.pair_type.into(),
            update_freq: old.update_freq,
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
    pub pair_type: OldFeedType,
    pub update_freq: Seconds,
    pub decimals: u64,
    pub status: OldFeedStatus,
    pub owner: Address,
    pub data: Option<RateDataLight>,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub enum OldFeedType {
    Custom {
        sources: Vec<Source>,
    },
    #[default]
    Default,
}


impl From<OldFeedType> for FeedType {
    fn from(old: OldFeedType) -> Self {
        match old {
            OldFeedType::Custom { sources } => FeedType::Custom { sources },
            OldFeedType::Default => FeedType::Default,
        }
    }
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


#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct OldState {
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Option<Principal>,
    pub key_name: String,
    pub mock: bool,
    pub pairs: OldFeedStorage,
    pub balances: Balances,
    pub balances_cfg: BalancesCfg,
    pub eth_address: Option<Address>,
    pub whitelist: Whitelist,
    pub data_fetchers: DataFetchersStorage,
    pub data_fetchers_indexer: DataFethcersIndexer,
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
            feeds: state.pairs.into(),
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
