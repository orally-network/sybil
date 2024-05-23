use std::collections::{HashMap, HashSet};

use candid::Nat;
use ic_cdk::{query, update};
use ic_web3_rs::{
    ethabi::{Event, EventParam, ParamType},
    types::{Log as TxLog, Transaction, TransactionReceipt, H160, H256, U256},
};
use lazy_static::lazy_static;
use sybil_utils::cycles_count;
use thiserror::Error;

pub const DECIMALS: u64 = 6;
pub const ETH_DECIMALS: u64 = 18;

use crate::{
    clone_with_state, log,
    types::{
        allowances::Allowances,
        balances::{
            AllowedChain, BalanceError, Balances, DepositError, ERC20Contract, SaveAllowedChain,
        },
        chains_rpc::RPCUrl,
        feed_types::get_xrc_data::GetXRCData,
        state::{self, get_cfg},
        whitelist::{Whitelist, WhitelistError},
    },
    utils::{
        address::{self, AddressError},
        canister, nat,
        siwe::{self, SiweError},
        validate_caller,
        web3::{self, Web3Error, SUCCESSFUL_TX_STATUS},
        CallerError,
    },
    STATE,
};

use super::feed_methods::get_xrc_data::_get_xrc_data;

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
    Web3(#[from] Web3Error),
    #[error("Caller error: {0}")]
    Caller(#[from] CallerError),
    #[error("Canister error: {0})")]
    Canister(#[from] canister::CanisterError),
    #[error("Allowed chain not found")]
    AllowedChainNotFound,
    #[error("Allowed chain already exists")]
    AllowedChainAlreadyExists,
}

#[query]
pub async fn get_allowed_chains() -> HashMap<u64, SaveAllowedChain> {
    state::get_cfg()
        .balances_cfg
        .allowed_chains
        .iter()
        .map(|(chain_id, allowed_chain)| (chain_id.clone(), allowed_chain.clone().into()))
        .collect()
}

#[query]
pub async fn get_treasure_address() -> String {
    state::get_cfg().balances_cfg.treasure_address.clone()
}

#[update]
pub async fn update_treasure_address(address: String) -> Result<(), String> {
    validate_caller().map_err(|_| format!("caller is not a controller"))?;
    let address = address::from_str(&address).map_err(|e| format!("invalid address: {}", e))?;
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.balances_cfg.treasure_address = address;
    });

    Ok(())
}

#[update]
pub async fn add_to_balances_whitelist(addresses: Vec<String>) -> Result<(), String> {
    validate_caller().map_err(|_| format!("caller is not a controller"))?;
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.balances_cfg.whitelist.extend(addresses);
    });

    Ok(())
}

#[update]
pub async fn remove_allowed_erc20_tokens(
    chain_id: u64,
    token_names: Vec<String>,
) -> Result<(), String> {
    _remove_allowed_erc20_tokens(chain_id, token_names)
        .await
        .map_err(|e| format!("failed to remove allowed erc20 token: {}", e))
}

#[inline(always)]
async fn _remove_allowed_erc20_tokens(
    chain_id: u64,
    token_names: Vec<String>,
) -> Result<(), BalancesError> {
    validate_caller()?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        let allowed_chain = state
            .balances_cfg
            .allowed_chains
            .get_mut(&chain_id)
            .ok_or(BalancesError::AllowedChainNotFound)?;

        allowed_chain
            .erc20_contracts
            .retain(|e| !token_names.contains(&e.token_symbol));

        Ok(())
    })
}

#[update]
pub async fn add_allowed_erc20_tokens(
    chain_id: u64,
    tokens: Vec<ERC20Contract>,
) -> Result<(), String> {
    _add_allowed_erc20_tokens(chain_id, tokens)
        .map_err(|e| format!("failed to add allowed erc20 token: {}", e))
}

#[inline(always)]
fn _add_allowed_erc20_tokens(
    chain_id: u64,
    tokens: Vec<ERC20Contract>,
) -> Result<(), BalancesError> {
    validate_caller()?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        let allowed_chain = state
            .balances_cfg
            .allowed_chains
            .get_mut(&chain_id)
            .ok_or(BalancesError::AllowedChainNotFound)?;

        allowed_chain.erc20_contracts.extend(tokens);

        Ok(())
    })
}

#[update]
pub async fn add_allowed_chain(
    chain_id: u64,
    rpc: String,
    coin_symbol: String,
    rpc_secret: Option<String>,
) -> Result<(), String> {
    _add_allowed_chain(chain_id, rpc, coin_symbol, rpc_secret)
        .await
        .map_err(|e| format!("failed to add allowed chain: {}", e))
}

#[inline(always)]
async fn _add_allowed_chain(
    chain_id: u64,
    rpc: String,
    coin_symbol: String,
    rpc_secret: Option<String>,
) -> Result<(), BalancesError> {
    validate_caller()?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        if state.balances_cfg.allowed_chains.contains_key(&chain_id) {
            return Err(BalancesError::AllowedChainAlreadyExists);
        }

        state.balances_cfg.allowed_chains.insert(
            chain_id,
            AllowedChain {
                rpc: RPCUrl {
                    url: rpc,
                    secret: rpc_secret,
                },
                coin_symbol,
                erc20_contracts: HashSet::new(),
            },
        );

        Ok(())
    })
}

#[update]
pub async fn remove_allowed_chain(chain_id: u64) -> Result<(), String> {
    _remove_allowed_chain(chain_id)
        .await
        .map_err(|e| format!("failed to add allowed chain: {}", e))
}

#[inline(always)]
async fn _remove_allowed_chain(chain_id: u64) -> Result<(), BalancesError> {
    validate_caller()?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        if !state.balances_cfg.allowed_chains.contains_key(&chain_id) {
            return Err(BalancesError::AllowedChainNotFound);
        }

        state.balances_cfg.allowed_chains.remove(&chain_id);
        Ok(())
    })
}

#[update]
pub async fn deposit(
    chain_id: u64,
    tx_hash: String,
    grantee: Option<String>, // Grant permissions to use this user's balance to this domain
    msg: String,
    sig: String,
) -> Result<(), String> {
    _deposit(chain_id, tx_hash, grantee, msg, sig)
        .await
        .map_err(|e| format!("deposit failed: {}", e))
}

#[inline(always)]
#[cycles_count]
async fn _deposit(
    chain_id: u64,
    tx_hash: String,
    grantee: Option<String>,
    msg: String,
    sig: String,
) -> Result<(), DepositError> {
    let caller = siwe::recover(&msg, &sig).await?;
    if !Whitelist::contains(&caller) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }
    let caller_eth = address::to_h160(&caller)?;

    let balances_cfg = state::get_cfg().balances_cfg;
    let allowed_chain = balances_cfg
        .allowed_chains
        .get(&chain_id)
        .ok_or(DepositError::ChainNotAllowed)?;

    let w3 = web3::instance(
        format!(
            "{}{}",
            clone_with_state!(rpc_wrapper),
            urlencoding::encode(&allowed_chain.rpc.get_url())
        ),
        clone_with_state!(evm_rpc_canister),
    );

    let tx_receipt = w3.get_tx_receipt(&tx_hash).await?;

    let tx = w3.get_tx(tx_receipt.transaction_hash.clone()).await?;

    // Deposit ERC20 tokens
    let erc20 = deposit_erc20(&tx_receipt, &caller_eth, &allowed_chain.erc20_contracts).await?;

    // Deposit native coint
    let eth = deposit_coin(&tx, &allowed_chain.coin_symbol).await?;

    let amount = erc20 + eth;

    if !Balances::contains(&caller) {
        Balances::add(&caller)?;
    }

    Balances::add_nonce(&caller, &nat::from_u256(&tx.nonce))?;
    Balances::add_amount(&caller, &nat::from_u256(&amount))?;

    if let Some(grantee) = grantee {
        Allowances::grant(grantee, caller.clone());
    }

    log!("[BALANCES] address {}, deposited {} usd", caller, amount);

    Ok(())
}

/// Check whether the transaction contains transfer of allowed ERC20 tokens
#[inline(always)]
async fn deposit_coin(tx: &Transaction, coin_symbol: &str) -> Result<U256, DepositError> {
    if tx.to.is_none()
        || tx.to.unwrap() != address::to_h160(&get_cfg().balances_cfg.treasure_address)?
    {
        return Ok(0.into());
    }

    let value_usd = if clone_with_state!(mock) || tx.value.is_zero() {
        tx.value
    } else {
        let xrc_data = _get_xrc_data(format!("{}/USD", coin_symbol), false, None, None).await?;

        let GetXRCData { rate, decimals, .. } = xrc_data.data;

        let mut usd = tx.value * Into::<U256>::into(rate);
        let mut decimals = decimals + ETH_DECIMALS;

        if decimals > DECIMALS {
            while decimals > DECIMALS {
                usd /= 10;
                decimals -= 1;
            }
        } else {
            while decimals < DECIMALS {
                usd *= 10;
                decimals += 1;
            }
        }

        usd
    };

    Ok(value_usd)
}

/// Check whether the transaction contains transfer of allowed ERC20 tokens
#[inline(always)]
async fn deposit_erc20(
    tx_receipt: &TransactionReceipt,
    caller: &H160,
    contracts: &HashSet<ERC20Contract>,
) -> Result<U256, DepositError> {
    let tx_status = tx_receipt
        .status
        .ok_or(DepositError::TxNotFinalized)?
        .as_u64();
    if tx_status != SUCCESSFUL_TX_STATUS {
        return Err(DepositError::TxFailed);
    }

    if &tx_receipt.from != caller {
        return Err(DepositError::CallerIsNotTxSender);
    }

    let to = tx_receipt.to.ok_or(DepositError::TxWithoutReceiver)?;

    let receiver = address::from_h160(&to)?;

    let Some(erc20_contract) = contracts.iter().find(|c| c.erc20_contract == receiver) else {
        return Ok(0.into());
    };

    let mut value_usd = U256::zero();

    for (event_from, event_to, value) in get_transfer_log(&tx_receipt.logs)? {
        validate_transfer_logs(&event_from, &event_to, &caller)?;

        value_usd += if clone_with_state!(mock) {
            value
        } else {
            let xrc_data = _get_xrc_data(
                format!("{}/USD", erc20_contract.token_symbol),
                false,
                None,
                None,
            )
            .await?;

            let GetXRCData { rate, decimals, .. } = xrc_data.data;

            let mut usd = value * Into::<U256>::into(rate);
            let mut decimals = erc20_contract.decimals + decimals;

            if decimals > DECIMALS {
                while decimals > DECIMALS {
                    usd /= 10;
                    decimals -= 1;
                }
            } else {
                while decimals < DECIMALS {
                    usd *= 10;
                    decimals += 1;
                }
            }

            usd
        };
    }

    Ok(value_usd)
}

#[inline(always)]
fn get_transfer_log(logs: &[TxLog]) -> Result<Vec<(H160, H160, U256)>, DepositError> {
    let transfer_logs: Vec<_> = logs
        .iter()
        .filter(|log| {
            log.topics
                .iter()
                .any(|topic| topic == &*TRANSFER_EVENT_SIGNATURE)
        })
        .collect();

    let mut res = Vec::with_capacity(transfer_logs.len());

    for log in transfer_logs {
        if log.topics.len() != 3 {
            return Err(DepositError::InvalidTransferEvent);
        }

        let from = H160::from_slice(&log.topics[1].0[12..]);
        let to = H160::from_slice(&log.topics[2].0[12..]);
        let value = U256::from_big_endian(&log.data.0);

        res.push((from, to, value));
    }

    Ok(res)
}

#[inline(always)]
fn validate_transfer_logs(from: &H160, to: &H160, caller: &H160) -> Result<(), DepositError> {
    if from != caller {
        return Err(DepositError::CallerIsNotTransferSender);
    }

    if address::from_h160(to)? != get_cfg().balances_cfg.treasure_address {
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

#[query]
pub fn get_base_fee() -> Nat {
    state::get_cfg().balances_cfg.base_fee
}

#[query]
pub fn get_fee_per_byte() -> Nat {
    state::get_cfg().balances_cfg.fee_per_byte
}

// TODO: delete
// #[update]
// pub async fn withdraw(
//     chain_id: u64,
//     amount: Nat,
//     to: String,
//     msg: String,
//     sig: String,
// ) -> Result<String, String> {
//     _withdraw(chain_id, amount, to, msg, sig)
//         .await
//         .map_err(|e| format!("withdraw failed: {}", e))
// }

// #[inline(always)]
// async fn _withdraw(
//     chain_id: u64,
//     amount: Nat,
//     to: String,
//     msg: String,
//     sig: String,
// ) -> Result<String, BalancesError> {
//     let caller = siwe::recover(&msg, &sig).await?;
//     let receiver = address::from_str(&to)?;
//     if !Whitelist::contains(&caller) {
//         return Err(WhitelistError::AddressNotWhitelisted.into());
//     }

//     if amount == 0 {
//         return Err(BalanceError::InsufficientBalance)?;
//     }

//     if !Balances::is_sufficient(&caller, &amount)? {
//         return Err(BalanceError::InsufficientBalance.into());
//     }

//     let cfg = state::get_cfg().balances_cfg;
//     let allowed_chain = cfg
//         .allowed_chains
//         .get(&chain_id)
//         .ok_or(BalancesError::ChainNotAllowed)?;

//     let w3 = web3::instance(
//         allowed_chain.rpc.clone(),
//         clone_with_state!(evm_rpc_canister),
//     );
//     let contract_addr =
//         Address::from_str(&cfg.erc20_contract).map_err(|_| AddressError::InvalidAddress)?;

//     let contract = Contract::from_json(w3.eth(), contract_addr, TOKEN_ABI)
//         .map_err(|err| BalancesError::Contract(err.to_string()))?;

//     let tx_hash = w3
//         .send_erc20(
//             &contract,
//             &amount,
//             &receiver,
//             canister::eth_address().await?.to_string(),
//             clone_with_state!(key_name),
//             nat::to_u64(&cfg.chain_id),
//         )
//         .await?;

//     Balances::reduce_amount(&caller, &amount)?;

//     log!("[BALANCES] address {}, withdrew {} tokens", caller, amount);
//     Ok(tx_hash)
// }

// #[update]
// pub async fn withdraw_fees(to: String) -> Result<String, String> {
//     _withdraw_fees(to)
//         .await
//         .map_err(|e| format!("withdraw fees failed: {}", e))
// }

// #[inline(always)]
// async fn _withdraw_fees(to: String) -> Result<String, BalancesError> {
//     validate_caller()?;
//     let receiver = address::from_str(&to)?;

//     let canister_addr = canister::eth_address().await?;

//     let fees = Balances::get_amount(&canister_addr)?;
//     if fees == 0 {
//         return Err(BalanceError::InsufficientBalance)?;
//     }

//     let cfg = state::get_cfg().balances_cfg;

//     let w3 = web3::instance(cfg.rpc, clone_with_state!(evm_rpc_canister));
//     let contract_addr =
//         Address::from_str(&cfg.erc20_contract).map_err(|_| AddressError::InvalidAddress)?;

//     let contract = Contract::from_json(w3.eth(), contract_addr, TOKEN_ABI)
//         .map_err(|err| BalancesError::Contract(err.to_string()))?;

//     let tx_hash = w3
//         .send_erc20(
//             &contract,
//             &fees,
//             &receiver,
//             canister::eth_address().await?.to_string(),
//             clone_with_state!(key_name),
//             nat::to_u64(&cfg.chain_id),
//         )
//         .await?;

//     Balances::reduce_amount(&canister_addr, &fees)?;

//     log!("[BALANCES] address {}, withdrew {} tokens", to, fees);
//     Ok(tx_hash)
// }
