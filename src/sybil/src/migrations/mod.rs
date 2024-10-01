#![allow(deprecated)]

use std::collections::{HashMap, HashSet};

use candid::{CandidType, Nat, Principal};
use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::{logger, monitor};
use serde::{Deserialize, Serialize};

use crate::{
    http::HttpService,
    log, metrics,
    types::{
        allowances::{Allowance, Allowances},
        api_keys::{APIKeys, User},
        balances::{AllowedChain, Balances, BalancesCfg, ERC20Contract},
        cache::{HttpCache, RateCache, SignaturesCache},
        chains_rpc::{ChainsRPC, RPCConfig, RPCUrl},
        dex_list::DEXList,
        feed_types::rate_data::AssetDataResult,
        feeds::{Feed, FeedStatus, FeedStorage, FeedType},
        http::APIRequest,
        source::{EvmEventLogsSource, HttpSource, Source},
        state::State,
        whitelist::Whitelist,
        Address, Seconds, Timestamp,
    },
    utils::{
        canister::set_custom_panic_hook,
        metrics::{Metric, Metrics, METRICS},
    },
    CACHE, HTTP_CACHE, HTTP_REQUESTS, SIGNATURES_CACHE, STATE,
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
                old.new_sources.map(|sources_vec| {
                    sources_vec
                        .into_iter()
                        .map(|source| source.into())
                        .collect()
                })
            },
            decimals: old.decimals,
            status: old.status.into(),
            owner: old.owner,
            data: old.data,
        }
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldEvmEventLogsSource {
    pub rpc: String,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub address: Option<String>,
    pub topic: Option<String>,
    pub block_hash: Option<String>,
    pub log_index: u32,
    pub event_log_field_name: String,
    pub event_name: String,
    pub event_abi: String,
}

impl From<OldEvmEventLogsSource> for EvmEventLogsSource {
    fn from(old: OldEvmEventLogsSource) -> Self {
        Self {
            rpc: old.rpc,
            from_block: old.from_block,
            to_block: old.to_block,
            address: old.address,
            topic: old.topic,
            log_index: old.log_index,
            event_log_field_name: old.event_log_field_name,
            event_name: old.event_name,
            event_abi: old.event_abi,
        }
    }
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum OldSource {
    HttpSource(HttpSource),
    EvmEventLogsSource(OldEvmEventLogsSource),
}

impl From<OldSource> for Source {
    fn from(old: OldSource) -> Self {
        match old {
            OldSource::HttpSource(s) => Source::HttpSource(s),
            OldSource::EvmEventLogsSource(logs) => Source::EvmEventLogsSource(logs.into()),
        }
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct OldFeed {
    pub id: String,
    pub feed_type: FeedType,
    pub update_freq: Seconds,
    pub sources: Option<Vec<HttpSource>>,
    pub new_sources: Option<Vec<OldSource>>,
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

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct OldBalancesCfg {
    pub rpc: String,
    pub chain_id: Nat,
    pub erc20_contract: Address,
    pub treasure_address: Option<Address>,
    pub fee_per_byte: Nat,
    pub base_fee: Option<Nat>,
    pub signature_fee: Option<Nat>,
    pub allowed_chains: Option<HashMap<u64, AllowedChain>>,
    // Vec of addresses that won't be charged for anything
    pub whitelist: Option<HashSet<String>>,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct OldAllowedChain {
    pub rpc: RPCUrl,
    pub coin_symbol: String,
    pub erc20_contracts: Option<HashSet<ERC20Contract>>,
}

impl From<OldAllowedChain> for AllowedChain {
    fn from(old: OldAllowedChain) -> Self {
        Self {
            rpc: old.rpc,
            coin_symbol: old.coin_symbol,
            erc20_contracts: old.erc20_contracts.unwrap_or_default(),
        }
    }
}

impl From<OldBalancesCfg> for BalancesCfg {
    fn from(old: OldBalancesCfg) -> Self {
        Self {
            rpc: old.rpc,
            chain_id: old.chain_id,
            erc20_contract: old.erc20_contract,
            allowed_chains: old.allowed_chains.unwrap_or_default(),
            treasure_address: old.treasure_address.unwrap_or_default(),
            fee_per_byte: old.fee_per_byte,
            base_fee: old.base_fee.unwrap_or_default(),
            signature_fee: old.signature_fee.unwrap_or_default(),
            whitelist: old.whitelist.unwrap_or_default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct OldUser {
    pub address: String,
    pub request_count: u64,
    pub request_count_today: Option<u64>,
    pub request_count_per_method: HashMap<String, u64>,
    pub request_count_per_domain: HashMap<String, u64>,
    pub banned_domains: HashSet<String>,
    pub allowed_domains: HashSet<String>,
    pub is_public: bool, // if true, everyone except banned domains can use this key, if false, only allowed domains can use this key
    pub last_request: u64, // timestamp of the last request
    pub request_limit_by_domain: u64,
    pub request_limit: u64,
}

impl From<OldUser> for User {
    fn from(old: OldUser) -> Self {
        Self {
            address: old.address,
            request_count: old.request_count,
            request_count_today: old.request_count_today.unwrap_or_default(),
            request_count_per_method: old.request_count_per_method,
            request_count_per_domain: old.request_count_per_domain,
            banned_domains: old.banned_domains,
            allowed_domains: old.allowed_domains,
            is_public: old.is_public,
            last_request: old.last_request,
            request_limit_by_domain: old.request_limit_by_domain,
            request_limit: old.request_limit,
        }
    }
}

#[derive(Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct OldAPIKeys {
    keys_to_user: HashMap<String, OldUser>,
    user_to_keys: HashMap<String, HashSet<String>>,
    free_request_limit: u64,
}

impl From<OldAPIKeys> for APIKeys {
    fn from(old: OldAPIKeys) -> Self {
        Self {
            keys_to_user: old
                .keys_to_user
                .into_iter()
                .map(|(key, user)| (key, user.into()))
                .collect(),
            user_to_keys: old.user_to_keys,
            free_request_limit: old.free_request_limit,
        }
    }
}

#[derive(Default, Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct OldAllowance {
    pub grantor_address: String,
    pub request_count: u64,
    pub request_count_today: u64,
    pub request_count_per_method: HashMap<String, u64>,
    pub request_count_per_domain: HashMap<String, u64>,
    pub request_limit: u64,
    pub last_request: u64, // timestamp of the last request
}

impl From<OldAllowance> for Allowance {
    fn from(value: OldAllowance) -> Self {
        Self {
            grantor_address: value.grantor_address,
            request_count: value.request_count,
            request_count_today: value.request_count_today,
            request_count_per_method: value.request_count_per_method,
            request_count_per_domain: value.request_count_per_domain,
            request_limit: value.request_limit,
            last_request: value.last_request,
        }
    }
}

#[derive(Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct OldAllowances {
    domains_to_allowances: HashMap<String, OldAllowance>,
    user_to_allowed_domains: HashMap<String, HashSet<String>>,
}

impl From<OldAllowances> for Allowances {
    fn from(value: OldAllowances) -> Self {
        Self {
            domains_to_allowances: value
                .domains_to_allowances
                .into_iter()
                .map(|(domain, allowance)| (domain, allowance.into()))
                .collect(),
            user_to_allowed_domains: value.user_to_allowed_domains,
        }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct OldRPCUrl {
    pub url: String,
    pub secret: Option<String>,
    pub config: Option<RPCConfig>,
}

impl From<OldRPCUrl> for RPCUrl {
    fn from(old: OldRPCUrl) -> Self {
        Self {
            url: old.url,
            secret: old.secret,
            config: old.config.unwrap_or_default(),
        }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct OldChainsRPC(pub HashMap<u64, Vec<RPCUrl>>);

impl From<OldChainsRPC> for ChainsRPC {
    fn from(old: OldChainsRPC) -> Self {
        Self(old.0.into_iter().collect())
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct OldState {
    pub api_keys: Option<OldAPIKeys>,
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Option<Principal>,
    pub evm_rpc_canister: Option<Principal>,
    pub rpc_wrapper: Option<String>,
    pub key_name: String,
    pub mock: bool,
    pub feeds: OldFeedStorage,
    pub balances: Balances,
    pub allowances: Option<OldAllowances>,
    pub balances_cfg: OldBalancesCfg,
    pub chains_rpc: Option<OldChainsRPC>,
    pub dex_list: Option<DEXList>,
    pub eth_address: Option<Address>,
    pub whitelist: Whitelist,
    pub data_fetchers: Option<DataFetchersStorage>,
    pub data_fetchers_indexer: Option<DataFethcersIndexer>,
    pub test: Option<u32>,
}

impl From<OldState> for State {
    fn from(state: OldState) -> Self {
        Self {
            api_keys: state.api_keys.map(|keys| keys.into()).unwrap_or_default(),
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
            allowances: state
                .allowances
                .map(|allowances| allowances.into())
                .unwrap_or_default(),
            balances_cfg: state.balances_cfg.into(),
            chains_rpc: state.chains_rpc.unwrap_or_default().into(),
            dex_list: state.dex_list.unwrap_or_default(),
            eth_address: state.eth_address,
            whitelist: state.whitelist,
            test: state.test.unwrap_or_default(),
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

    let http_requests = HTTP_REQUESTS.with(|http_requests| http_requests.borrow().clone());

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
        http_requests,
        metrics,
    ))
    .expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (
        state,
        cache,
        log_data,
        monitor_data,
        http_cache,
        signatures_cache,
        http_requests,
        metrics,
    ): (
        OldState,
        OldRateCache,
        logger::PostUpgradeStableData,
        monitor::PostUpgradeStableData,
        HttpCache,
        SignaturesCache,
        Option<HashMap<String, APIRequest>>,
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
    HTTP_REQUESTS.with(|c| {
        if let Some(http_requests) = http_requests {
            c.replace(http_requests);
        }
    });

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
