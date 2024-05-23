use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use candid::{CandidType, Nat};
use ic_web3_rs::ethabi::Error as EthabiError;

use super::{
    allowances::AllowancesError, chains_rpc::RPCUrl, feeds::FeedError, whitelist::WhitelistError,
    Address,
};
use crate::{
    clone_with_state,
    utils::{address::AddressError, canister::CanisterError, siwe::SiweError, web3::Web3Error},
    STATE,
};

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct BalancesCfg {
    #[deprecated]
    pub rpc: String,
    #[deprecated]
    pub chain_id: Nat,
    #[deprecated]
    pub erc20_contract: Address,
    // Allowed chains with their ERC20 contracts that allowed to be deposited
    pub allowed_chains: HashMap<u64, AllowedChain>,
    pub treasure_address: Address,
    pub fee_per_byte: Nat,
    pub base_fee: Nat,
    // Vec of addresses that won't be charged for anything
    #[serde(default)]
    pub whitelist: HashSet<String>,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct UpdateBalancesCfg {
    pub treasure_address: Option<Address>,
    pub fee_per_byte: Option<Nat>,
    pub base_fee: Option<Nat>,
}

impl BalancesCfg {
    pub fn update(&mut self, cfg: &UpdateBalancesCfg) {
        if let Some(treasure_address) = &cfg.treasure_address {
            self.treasure_address = treasure_address.clone();
        }

        if let Some(fee_per_byte) = &cfg.fee_per_byte {
            self.fee_per_byte = fee_per_byte.clone();
        }

        if let Some(base_fee) = &cfg.base_fee {
            self.base_fee = base_fee.clone();
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ERC20Contract {
    pub erc20_contract: String,
    pub token_symbol: String,
    pub decimals: u64,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct AllowedChain {
    pub rpc: RPCUrl,
    pub coin_symbol: String,
    pub erc20_contracts: HashSet<ERC20Contract>,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct SaveAllowedChain {
    pub rpc: String,
    pub coin_symbol: String,
    pub erc20_contracts: HashSet<ERC20Contract>,
}

impl From<AllowedChain> for SaveAllowedChain {
    fn from(allowed_chain: AllowedChain) -> Self {
        Self {
            rpc: allowed_chain.rpc.to_string_with_access(),
            coin_symbol: allowed_chain.coin_symbol.clone(),
            erc20_contracts: allowed_chain.erc20_contracts.clone(),
        }
    }
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
    #[error("Allowances error: {0}")]
    AllowancesError(#[from] AllowancesError),
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
    #[error("This chain is not allowed for deposit")]
    ChainNotAllowed,
    #[error("Feed error: {0}")]
    FeedError(#[from] FeedError),
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

            if balance.amount < amount.clone() {
                return Err(BalanceError::InsufficientBalance);
            }

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
