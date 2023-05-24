use ic_cdk::{post_upgrade, pre_upgrade, storage};
use ic_utils::monitor;

use crate::{
    types::{cache::Cache, state::State},
    CACHE, STATE,
};

#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|state| state.borrow().clone());
    let cache = CACHE.with(|cache| cache.borrow().clone());

    let monitor_data = monitor::pre_upgrade_stable_data();

    storage::stable_save((state, cache, monitor_data)).expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, cache, monitor_data): (State, Cache, monitor::PostUpgradeStableData) =
        storage::stable_restore().expect("should be able to restore");

    monitor::post_upgrade_stable_data(monitor_data);

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache));
}
