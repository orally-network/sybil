use thiserror::Error;

use ic_web3_rs::ic::get_eth_addr;

use super::address::{self, AddressError};
use crate::{
    clone_with_state,
    types::{
        balances::{BalanceError, Balances},
        Address,
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

    let key_name = clone_with_state!(key_name);

    let raw_address = get_eth_addr(None, None, key_name)
        .await
        .map_err(CanisterError::UnableToGetEthAddress)?;

    let formatted_address = address::from_h160(&raw_address)?;

    update_state!(eth_address, Some(formatted_address.clone()));
    Balances::add(&formatted_address)?;

    Ok(formatted_address)
}
