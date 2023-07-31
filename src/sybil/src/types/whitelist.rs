use std::collections::HashSet;

use thiserror::Error;

use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::Address;

use crate::STATE;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Whitelist(HashSet<Address>);

#[derive(Error, Debug)]
pub enum WhitelistError {
    #[error("Address is already whitelisted")]
    AddressAlreadyWhitelisted,
    #[error("Address is not whitelisted")]
    AddressNotWhitelisted,
}

impl Whitelist {
    pub fn add(address: Address) -> Result<(), WhitelistError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if !state.whitelist.0.insert(address) {
                return Err(WhitelistError::AddressAlreadyWhitelisted);
            }

            Ok(())
        })
    }

    pub fn remove(address: &Address) -> Result<(), WhitelistError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if !state.whitelist.0.remove(address) {
                return Err(WhitelistError::AddressNotWhitelisted);
            }

            Ok(())
        })
    }

    pub fn contains(address: &Address) -> bool {
        STATE.with(|state| {
            let state = state.borrow();
            state.whitelist.0.contains(address)
        })
    }

    pub fn get_all() -> Vec<Address> {
        STATE.with(|state| {
            let state = state.borrow();
            state.whitelist.0.iter().cloned().collect()
        })
    }

    pub fn clear() {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.whitelist.0.clear();
        })
    }
}
