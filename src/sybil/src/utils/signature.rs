use anyhow::{anyhow, Result};

use ic_cdk::update;
use ic_web3_rs::{
    ic::{get_eth_addr, recover_address},
    types::H160,
};

use crate::STATE;

pub fn get_eth_v(signature: &[u8], message: &[u8], public_key: &H160) -> Result<u8> {
    let pub_key = hex::encode(public_key);

    let mut recovered_address = recover_address(message.to_vec(), signature.to_vec(), 0);
    if recovered_address == pub_key {
        return Ok(27);
    }

    recovered_address = recover_address(message.to_vec(), signature.to_vec(), 1);
    if recovered_address == pub_key {
        return Ok(28);
    }

    Err(anyhow!("invalid signature"))
}

#[update]
pub async fn eth_address() -> Result<String, String> {
    let key_name = STATE.with(|state| state.borrow().key_name.clone());

    match get_eth_addr(None, None, key_name).await {
        Ok(eth_addr) => Ok(hex::encode(eth_addr)),
        Err(err) => Err(err),
    }
}
