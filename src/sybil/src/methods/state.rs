use crate::{STATE, utils::validate_caller};
use candid::{Nat, Principal};
use ic_cdk::{query, update, api::management_canister::{provisional::CanisterIdRecord, main::canister_status}};

#[update]
pub fn set_exchange_rate_canister(new_principal: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| {
        state.borrow_mut().exchange_rate_canister = new_principal;
    });
}

#[query]
pub fn get_exchange_rate_canister() -> String {
    STATE.with(|state| state.borrow().exchange_rate_canister.clone())
}

#[update]
pub fn set_proxy_ecdsa_canister(new_principal: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| {
        state.borrow_mut().proxy_ecdsa_canister = new_principal;
    });
}

#[query]
pub fn get_proxy_ecdsa_canister() -> String {
    STATE.with(|state| state.borrow().proxy_ecdsa_canister.clone())
}

#[update]
pub fn set_siwe_signer_canister(canister: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| state.borrow_mut().siwe_signer_canister = canister)
}

#[query]
pub fn get_siwe_signer_canister() -> String {
    STATE.with(|state| state.borrow().siwe_signer_canister.clone())
}

#[update]
pub fn set_expiration_time(expiration_time: Nat) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| {
        state.borrow_mut().cache_expiration = *expiration_time
            .0
            .to_u64_digits()
            .last()
            .expect("expiration time should have at least one number")
    })
}

#[query]
pub fn get_expiration_time() -> Nat {
    STATE.with(|state| state.borrow().cache_expiration.into())
}

#[update]
pub fn set_key_name(key_name: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }
    
    STATE.with(|state| state.borrow_mut().key_name = key_name)
}

#[query]
pub fn get_key_name() -> String {
    STATE.with(|state| state.borrow().key_name.clone())
}

#[query]
pub fn get_treasurer_canister() -> String {
    STATE.with(|state| state.borrow().treasurer_canister.clone())
}

#[update]
pub fn set_treasurer_canister(canister: String) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| state.borrow_mut().treasurer_canister = canister)
}

#[update]
pub fn set_cost_per_execution(cost: Nat) {
    if validate_caller().is_err() {
        ic_cdk::trap("invalid caller")
    }

    STATE.with(|state| {
        state.borrow_mut().cost_per_execution = *cost
            .0
            .to_u64_digits()
            .last()
            .expect("cost should have at least one number")
    })
}

#[query]
pub fn get_cost_per_execution() -> Nat {
    STATE.with(|state| state.borrow().cost_per_execution.into())
}

#[update]
pub async fn init_controllers() -> Vec<Principal> {
    let canister_id_record = CanisterIdRecord {
        canister_id: ic_cdk::id(),
    };

    let (canister_status,) = canister_status(canister_id_record)
        .await
        .expect("should execute in the IC environment");

    STATE.with(|state| {
        state.borrow_mut().controllers = canister_status.settings.controllers;
    });

    STATE.with(|state| state.borrow().controllers.clone())
}