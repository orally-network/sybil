use thiserror::Error;

use super::address::AddressError;
use crate::{
    clone_with_state, log,
    types::{
        balances::{BalanceError, Balances},
        state, Address,
    },
    update_state,
};

#[derive(Error, Debug)]
pub enum CanisterError {
    #[error("unable to get eth address: {0}")]
    UnableToGetEthAddress(String),
    #[error("address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("balance error: {0}")]
    BalanceError(#[from] BalanceError),
}

pub async fn eth_address() -> Result<Address, CanisterError> {
    if let Some(address) = clone_with_state!(eth_address) {
        return Ok(address);
    }

    // let raw_address = get_eth_addr(None, None, key_name)
    //     .await
    //     .map_err(CanisterError::UnableToGetEthAddress)?;

    // let formatted_address = address::from_h160(&raw_address)?;

    let formatted_address = state::get_cfg().balances_cfg.treasure_address;

    update_state!(eth_address, Some(formatted_address.clone()));
    Balances::add(&formatted_address)?;

    Ok(formatted_address)
}

pub fn set_custom_panic_hook() {
    _ = std::panic::take_hook(); // clear custom panic hook and set default
    let old_handler = std::panic::take_hook(); // take default panic hook

    // set custom panic hook
    std::panic::set_hook(Box::new(move |info| {
        log!("PANIC OCCURRED: {:#?}", info);
        old_handler(info);
    }));
}
