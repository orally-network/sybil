use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use candid::{CandidType, Nat};
use ic_web3_rs::ethabi::Error as EthabiError;

use super::{whitelist::WhitelistError, Address};
use crate::{
    clone_with_state,
    utils::{address::AddressError, canister::CanisterError, siwe::SiweError, web3::Web3Error},
    STATE,
};

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct BalancesCfg {
    pub rpc: String,
    pub chain_id: Nat,
    pub erc20_contract: Address,
    pub fee_per_byte: Nat,
    // Vec of addresses that won't be charged for anything
    #[serde(default)]
    pub whitelist: HashSet<String>,
}

#[derive(Error, Debug)]
pub enum BalanceError {
    #[error("balance already exists")]
    BalanceAlreadyExists,
    #[error("balance does not exist")]
    BalanceDoesNotExist,
    #[error("nonce already used")]
    NonceAlreadyUsed,
    #[error("insufficient balance")]
    InsufficientBalance,
}

#[derive(Error, Debug)]
pub enum DepositError {
    #[error("balance error: {0}")]
    BalanceError(#[from] BalanceError),
    #[error("web3 error: {0}")]
    Web3Error(#[from] Web3Error),
    #[error("SIWE Error: {0}")]
    SIWEError(#[from] SiweError),
    #[error("tx is not finalized")]
    TxNotFinalized,
    #[error("tx has failed")]
    TxFailed,
    #[error("address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("caller is not tx sender")]
    CallerIsNotTxSender,
    #[error("tx without receiver")]
    TxWithoutReceiver,
    #[error("tx was not sent to the erc20 contract")]
    TxNotSentToErc20Contract,
    #[error("tx without transfer event")]
    TxWithoutTransferEvent,
    #[error("transfer log has invalid format: {0}")]
    TransferLogInvalidFormat(#[from] EthabiError),
    #[error("caller is not the sender of the transfer")]
    CallerIsNotTransferSender,
    #[error("unable to get canister eth address: {0}")]
    UnableToGetCanisterEthAddress(#[from] CanisterError),
    #[error("token receiver is not the canister eth address")]
    TokenReceiverIsNotCanisterEthAddress,
    #[error("Whitelist error: {0}")]
    Whitelist(#[from] WhitelistError),
    #[error("invalid transfer event")]
    InvalidTransferEvent,
}

#[derive(Debug, CandidType, Deserialize, Serialize, Default, Clone)]
pub struct BalanceEntry {
    pub amount: Nat,
    pub nonces: Vec<Nat>,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
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

    pub fn add_amount(address: &Address, amount: &Nat) -> Result<(), BalanceError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let balance = state
                .balances
                .0
                .get_mut(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?;

            balance.amount += amount.clone();

            Ok(())
        })
    }

    pub fn add_nonce(address: &Address, nonce: &Nat) -> Result<(), BalanceError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let nonces = &mut state
                .balances
                .0
                .get_mut(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?
                .nonces;

            if nonces.contains(nonce) {
                return Err(BalanceError::NonceAlreadyUsed);
            }

            nonces.push(nonce.clone());

            Ok(())
        })
    }

    pub fn contains(address: &Address) -> bool {
        STATE.with(|state| state.borrow().balances.0.contains_key(address))
    }

    pub fn is_sufficient(address: &Address, amount: &Nat) -> Result<bool, BalanceError> {
        if clone_with_state!(balances_cfg)
            .whitelist
            .contains(&address.to_string())
        {
            return Ok(true);
        }

        STATE.with(|state| {
            let state = state.borrow();
            let balance = state
                .balances
                .0
                .get(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?;

            Ok(&balance.amount >= amount)
        })
    }

    pub fn reduce_amount(address: &Address, amount: &Nat) -> Result<(), BalanceError> {
        if clone_with_state!(balances_cfg)
            .whitelist
            .contains(&address.to_string())
        {
            return Ok(());
        }

        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let balance = state
                .balances
                .0
                .get_mut(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?;

            balance.amount -= amount.clone();

            Ok(())
        })
    }

    pub fn clear() {
        STATE.with(|state| {
            state.borrow_mut().balances.0.clear();
        })
    }
}
