use crate::STATE;
use candid::Nat;
use ic_cdk::{query, update};

#[update]
pub fn set_exchange_rate_canister(new_principal: String) {
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
    STATE.with(|state| state.borrow_mut().siwe_signer_canister = canister)
}

#[query]
pub fn get_siwe_signer_canister() -> String {
    STATE.with(|state| state.borrow().siwe_signer_canister.clone())
}

#[update]
pub fn set_expiration_time(expiration_time: Nat) {
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
    STATE.with(|state| state.borrow_mut().key_name = key_name)
}

#[query]
pub fn get_key_name() -> String {
    STATE.with(|state| state.borrow().key_name.clone())
}
