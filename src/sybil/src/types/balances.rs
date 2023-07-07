use std::collections::HashMap;

use thiserror::Error;

use candid::{Nat, CandidType};
use ic_cdk::export::serde::{Deserialize, Serialize};

use super::Address;
use crate::{STATE, utils::web3::Web3Error};

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct BalancesCfg {
    pub rpc: String,
    pub chain_id: Nat,
    pub erc20_contract: Address,
}

#[derive(Error, Debug)]
pub enum BalanceError {
    #[error("balance already exists")]
    BalanceAlreadyExists,
    #[error("balance does not exist")]
    BalanceDoesNotExist,
}

#[derive(Error, Debug)]
pub enum DepositError {
    #[error("invalid tx hash")]
    InvalidTxHash,
    #[error("balance error: {0}")]
    BalanceError(#[from] BalanceError),
    #[error("web3 error: {0}")]
    Web3Error(#[from] Web3Error),
    #[error("tx does not exist")]
    TxDoesNotExist,
}


#[derive(CandidType, Deserialize, Serialize, Default, Clone)]
pub struct BalanceEntry {
    pub amount: Nat,
    pub nonces: Vec<Nat>
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone)]
pub struct Balances(HashMap<Address, BalanceEntry>);

impl Balances {
    pub fn add(address: &Address) -> Result<(), BalanceError> {
        STATE.with(|state| {
            let balances = &mut state.borrow_mut().balances;
            
            if balances.0.contains_key(address) {
                return Err(BalanceError::BalanceAlreadyExists);
            }
            
            balances.0.insert(address.clone(), BalanceEntry::default());

            Ok(())
        })
    }

    pub fn remove(address: &Address) -> Result<(), BalanceError> {
        STATE.with(|state| {
            let balances = &mut state.borrow_mut().balances;
            
            if !balances.0.contains_key(address) {
                return Err(BalanceError::BalanceDoesNotExist);
            }
            
            balances.0.remove(address);

            Ok(())
        })
    }

    pub fn get_amount(address: &Address) -> Result<Nat, BalanceError> {
        STATE.with(|state| {
            Ok(state
                .borrow()
                .balances
                .0
                .get(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?
                .amount
                .clone())
        })
    }

    pub fn add_nonce(address: &Address, nonce: &Nat) -> Result<(), BalanceError> {
        STATE.with(|state| {
            state
                .borrow_mut()
                .balances
                .0
                .get_mut(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?
                .nonces
                .push(nonce.clone());

            Ok(())
        })
    }
}
