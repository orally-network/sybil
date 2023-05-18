use ic_cdk::{post_upgrade, pre_upgrade, storage};

use crate::{
    types::{cache::Cache, state::State},
    CACHE, STATE,
};

#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|state| state.borrow().clone());
    let cache = CACHE.with(|cache| cache.borrow().clone());

    storage::stable_save((state, cache)).expect("should be able to save");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, cache): (State, Cache) =
        storage::stable_restore().expect("should be able to restore");

    STATE.with(|s| s.replace(state));
    CACHE.with(|c| c.replace(cache));
}
