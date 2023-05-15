use crate::STATE;
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
