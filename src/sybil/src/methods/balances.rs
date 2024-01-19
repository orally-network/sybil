use std::str::FromStr;

use candid::Nat;
use ic_cdk::{query, update};
use ic_web3_rs::{
    ethabi::{Event, EventParam, ParamType},
    types::{Log as TxLog, TransactionReceipt, H160, H256, U256},
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    log,
    types::{
        balances::{BalanceError, Balances, DepositError},
        state,
        whitelist::{Whitelist, WhitelistError},
    },
    utils::{
        address::{self, AddressError},
        canister, nat,
        siwe::{self, SiweError},
        validate_caller, web3, CallerError,
    },
};

lazy_static! {
    static ref TRANSFER_EVENT: Event = Event {
        name: "Transfer".into(),
        inputs: vec![
            EventParam {
                name: "_from".into(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "_to".into(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "_value".into(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    };
    static ref TRANSFER_EVENT_SIGNATURE: H256 = TRANSFER_EVENT.signature();
}

#[derive(Error, Debug)]
pub enum BalancesError {
    #[error("Address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("Balance error: {0}")]
    BalanceError(#[from] BalanceError),
    #[error("SIWE error: {0}")]
    Siwe(#[from] SiweError),
    #[error("Whitelist error: {0}")]
    Whitelist(#[from] WhitelistError),
    #[error("Web3 error: {0}")]
    Web3(#[from] web3::Web3Error),
    #[error("Caller error: {0}")]
    Caller(#[from] CallerError),
    #[error("Canister error: {0})")]
    Canister(#[from] canister::CanisterError),
}

#[update]
pub async fn deposit(tx_hash: String, msg: String, sig: String) -> Result<(), String> {
    _deposit(tx_hash, msg, sig)
        .await
        .map_err(|e| format!("deposit failed: {}", e))
}

#[inline(always)]
async fn _deposit(tx_hash: String, msg: String, sig: String) -> Result<(), DepositError> {
    let caller = siwe::recover(&msg, &sig).await?;
    if !Whitelist::contains(&caller) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }
    let caller_eth = address::to_h160(&caller)?;
    let tx_hash = H256::from_str(&tx_hash).map_err(|_| DepositError::InvalidTxHash)?;

    let balances_cfg = state::get_cfg().balances_cfg;
    let w3 = &web3::client(&balances_cfg.rpc);

    let tx_receipt = web3::tx_receipt(w3, &tx_hash)
        .await?
        .ok_or(DepositError::TxDoesNotExist)?;
    validate_deposit_tx_receipt(
        &tx_receipt,
        &caller_eth,
        &address::to_h160(&balances_cfg.erc20_contract)?,
    )?;

    let tx = web3::tx_by_hash(w3, &tx_hash)
        .await?
        .ok_or(DepositError::TxDoesNotExist)?;

    let (event_from, event_to, value) = get_transfer_log(&tx_receipt.logs)?;
    validate_transfer_log(
        &event_from,
        &event_to,
        &caller_eth,
        &address::to_h160(&canister::eth_address().await?)?,
    )?;

    if !Balances::contains(&caller) {
        Balances::add(&caller)?;
    }

    Balances::add_nonce(&caller, &nat::from_u256(&tx.nonce))?;
    Balances::add_amount(&caller, &nat::from_u256(&value))?;

    log!("[BALANCES] address {}, deposited {} tokens", caller, value);
    Ok(())
}

#[inline(always)]
fn get_transfer_log(logs: &[TxLog]) -> Result<(H160, H160, U256), DepositError> {
    let log = logs
        .iter()
        .find(|log| {
            log.topics
                .iter()
                .any(|topic| topic == &*TRANSFER_EVENT_SIGNATURE)
        })
        .ok_or(DepositError::TxWithoutTransferEvent)?;

    if log.topics.len() != 3 {
        return Err(DepositError::InvalidTransferEvent);
    }

    let from = H160::from_slice(&log.topics[1].0[12..]);
    let to = H160::from_slice(&log.topics[2].0[12..]);
    let value = U256::from_big_endian(&log.data.0);

    Ok((from, to, value))
}

#[inline(always)]
fn validate_deposit_tx_receipt(
    tx_receipt: &TransactionReceipt,
    caller: &H160,
    contract: &H160,
) -> Result<(), DepositError> {
    let tx_status = tx_receipt
        .status
        .ok_or(DepositError::TxNotFinalized)?
        .as_u64();
    if tx_status != web3::SUCCESSFUL_TX_STATUS {
        return Err(DepositError::TxFailed);
    }

    if &tx_receipt.from != caller {
        return Err(DepositError::CallerIsNotTxSender);
    }

    let to = tx_receipt.to.ok_or(DepositError::TxWithoutReceiver)?;
    if &to != contract {
        return Err(DepositError::TxNotSentToErc20Contract);
    }

    Ok(())
}

#[inline(always)]
fn validate_transfer_log(
    from: &H160,
    to: &H160,
    caller: &H160,
    canister_eth_address: &H160,
) -> Result<(), DepositError> {
    if from != caller {
        return Err(DepositError::CallerIsNotTransferSender);
    }

    if to != canister_eth_address {
        return Err(DepositError::TokenReceiverIsNotCanisterEthAddress);
    }

    Ok(())
}

#[query]
pub fn get_balance(addr: String) -> Result<Nat, String> {
    _get_balance(addr).map_err(|e| format!("get balance failed: {}", e))
}

#[inline(always)]
fn _get_balance(addr: String) -> Result<Nat, BalancesError> {
    Ok(Balances::get_amount(&address::from_str(&addr)?).unwrap_or_default())
}

#[update]
pub async fn withdraw(amount: Nat, to: String, msg: String, sig: String) -> Result<String, String> {
    _withdraw(amount, to, msg, sig)
        .await
        .map_err(|e| format!("withdraw failed: {}", e))
}

#[inline(always)]
async fn _withdraw(
    amount: Nat,
    to: String,
    msg: String,
    sig: String,
) -> Result<String, BalancesError> {
    let caller = siwe::recover(&msg, &sig).await?;
    let receiver = address::from_str(&to)?;
    if !Whitelist::contains(&caller) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    if amount == 0 {
        return Err(BalanceError::InsufficientBalance)?;
    }

    if !Balances::is_sufficient(&caller, &amount)? {
        return Err(BalanceError::InsufficientBalance.into());
    }

    let tx_hash = web3::send_erc20(&amount, &receiver).await?;

    Balances::reduce_amount(&caller, &amount)?;

    log!("[BALANCES] address {}, withdrew {} tokens", caller, amount);
    Ok(tx_hash)
}

#[update]
pub async fn withdraw_fees(to: String) -> Result<String, String> {
    _withdraw_fees(to)
        .await
        .map_err(|e| format!("withdraw fees failed: {}", e))
}

#[inline(always)]
async fn _withdraw_fees(to: String) -> Result<String, BalancesError> {
    validate_caller()?;
    let receiver = address::from_str(&to)?;

    let canister_addr = canister::eth_address().await?;

    let fees = Balances::get_amount(&canister_addr)?;
    if fees == 0 {
        return Err(BalanceError::InsufficientBalance)?;
    }

    let tx_hash = web3::send_erc20(&fees, &receiver).await?;

    Balances::reduce_amount(&canister_addr, &fees)?;

    log!("[BALANCES] address {}, withdrew {} tokens", to, fees);
    Ok(tx_hash)
}
