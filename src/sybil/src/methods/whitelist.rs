use ic_cdk::{query, update};

use thiserror::Error;

use crate::{
    log,
    types::{
        whitelist::{Whitelist, WhitelistError},
        Address,
    },
    utils::{
        address::{self, AddressError},
        validate_caller, CallerError,
    },
};

#[derive(Error, Debug)]
pub enum WhitelistRequestsError {
    #[error("Whitelist error: {0}")]
    Whitelist(#[from] WhitelistError),
    #[error("Caller error: {0}")]
    Caller(#[from] CallerError),
    #[error("Address error: {0}")]
    Address(#[from] AddressError),
}

#[update]
pub fn add_to_whitelist(addr: Address) -> Result<(), String> {
    _add_to_whitelist(addr).map_err(|e| format!("failed to add an address to the whitelist: {e}"))
}

#[inline(always)]
fn _add_to_whitelist(addr: Address) -> Result<(), WhitelistRequestsError> {
    validate_caller()?;
    Whitelist::add(address::from_str(&addr)?)?;

    log!("[WHITELIST] user added to the whitelist. Address: {addr}");
    Ok(())
}

#[update]
pub fn remove_from_whitelist(addr: Address) -> Result<(), String> {
    _remove_from_whitelist(addr)
        .map_err(|e| format!("failed to remove an address from the whitelist: {e}"))
}

#[inline(always)]
fn _remove_from_whitelist(addr: Address) -> Result<(), WhitelistRequestsError> {
    validate_caller()?;
    Whitelist::remove(&address::from_str(&addr)?)?;

    log!("[WHITELIST] user removed from the whitelist. Address: {addr}");
    Ok(())
}

#[query]
pub fn is_whitelisted(addr: Address) -> Result<bool, String> {
    _is_whitelisted(addr).map_err(|e| format!("failed to check if an address is whitelisted: {e}"))
}

#[inline(always)]
pub fn _is_whitelisted(addr: Address) -> Result<bool, WhitelistRequestsError> {
    Ok(Whitelist::contains(&address::from_str(&addr)?))
}

#[query]
pub fn get_whitelist() -> Result<Vec<Address>, String> {
    _get_whitelist().map_err(|e| format!("failed to get the whitelist: {e}"))
}

#[inline(always)]
pub fn _get_whitelist() -> Result<Vec<Address>, WhitelistRequestsError> {
    validate_caller()?;
    let whiltelist = Whitelist::get_all();

    Ok(whiltelist)
}
