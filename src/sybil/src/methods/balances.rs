use std::str::FromStr;

use ic_cdk::update;
use ic_web3_rs::types::H256;

use crate::{utils::web3, types::{state, balances::DepositError}};

#[update]
pub async fn deposit(tx_hash: String) -> Result<(), String> {
    _deposit(tx_hash)
        .await
        .map_err(|e| format!("deposit failed: {:?}", e))
}

#[inline(always)]
async fn _deposit(tx_hash: String) -> Result<(), DepositError> {
    let tx_hash = H256::from_str(&tx_hash)
        .map_err(|_| DepositError::InvalidTxHash)?;
    
    let balances_cfg = state::get_cfg().balances_cfg;

    let w3 = &web3::client(&balances_cfg.rpc);

    let _ = web3::tx_receipt(w3, &tx_hash)
        .await?
        .ok_or(DepositError::TxDoesNotExist)?;
    


    Ok(())
}
