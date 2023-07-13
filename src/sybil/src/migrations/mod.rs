use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::monitor;

use crate::{
    types::{
        cache::{HTTPCache, RateCache, SignaturesCache},
        state::State,
    },
    CACHE, HTTP_CACHE, SIGNATURES_CACHE, STATE,
};

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
        State,
        RateCache,
        monitor::PostUpgradeStableData,
        HTTPCache,
        SignaturesCache,
    ) = storage::stable_restore().expect("should be able to restore");

    monitor::post_upgrade_stable_data(monitor_data);

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache));
    HTTP_CACHE.with(|c| c.replace(http_cache));
    SIGNATURES_CACHE.with(|c| c.replace(signatures_cache));
}
