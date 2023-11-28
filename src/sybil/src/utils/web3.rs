use std::str::FromStr;

use candid::Nat;
use ic_web3_rs::{
    contract::{tokens::Tokenizable, Contract, Options},
    ethabi::{Address, Error as EthabiError, Token},
    ic::KeyInfo,
    transports::ICHttp,
    types::{Transaction, TransactionId, TransactionReceipt, H256},
    Error as Web3ClientError, Web3,
};

use thiserror::Error;

use super::{address, canister, nat, processors};
use crate::{clone_with_state, retry_until_success, types::state};

pub const SUCCESSFUL_TX_STATUS: u64 = 1;
pub const TOKEN_ABI: &[u8] = include_bytes!("../assets/ERC20ABI.json");
pub const ECDSA_SIGN_CYCLES: u64 = 23_000_000_000;
pub const ERC20_TRANSFER_METHOD: &str = "transfer";

#[derive(Error, Debug)]
pub enum Web3Error {
    #[error("client error: {0}")]
    ClientError(#[from] Web3ClientError),
    #[error("address error: {0}")]
    Address(#[from] address::AddressError),
    #[error("canister error: {0}")]
    Canister(#[from] canister::CanisterError),
    #[error("Contract error: {0}")]
    Contract(String),
    #[error("ethabi error: {0}")]
    Ethabi(#[from] EthabiError),
}

#[inline(always)]
pub fn client(rpc: &str) -> Web3<ICHttp> {
    Web3::new(ICHttp::new(rpc, None)
        .expect("In this version of the ic web3 client this method will never fail, if it does, discart this version and use the one from the git repo of mine"))
}

pub async fn tx_receipt(
    w3: &Web3<ICHttp>,
    tx_hash: &H256,
) -> Result<Option<TransactionReceipt>, Web3Error> {
    Ok(w3
        .eth()
        .transaction_receipt(*tx_hash, processors::transform_ctx_tx_with_logs())
        .await?)
}

pub async fn tx_by_hash(
    w3: &Web3<ICHttp>,
    tx_hash: &H256,
) -> Result<Option<Transaction>, Web3Error> {
    let tx_id = TransactionId::Hash(*tx_hash);

    Ok(w3
        .eth()
        .transaction(tx_id, processors::transform_ctx_tx())
        .await?)
}

pub async fn send_erc20(value: &Nat, to: &str) -> Result<String, Web3Error> {
    let sybil_addr = canister::eth_address().await?;
    let from = address::to_h160(&sybil_addr)?;

    let cfg = state::get_cfg().balances_cfg;

    let w3 = Web3::new(ICHttp::new(&cfg.rpc, None)?);
    let contract_addr =
        Address::from_str(&cfg.erc20_contract).map_err(|e| Web3Error::Contract(format!("{e}")))?;
    let contract = Contract::from_json(w3.eth(), contract_addr, TOKEN_ABI)?;

    let value = nat::to_u256(value);

    let tx_count =
        retry_until_success!(w3
            .eth()
            .transaction_count(from, None, processors::transform_ctx()))?;
    let gas_price = retry_until_success!(w3.eth().gas_price(processors::transform_ctx()))?;
    let options = Options::with(|op| {
        op.nonce = Some(tx_count);
        op.gas_price = Some(gas_price);
    });

    let key_name = clone_with_state!(key_name);

    let key_info = KeyInfo {
        derivation_path: vec![ic_cdk::id().as_slice().to_vec()],
        key_name,
        ecdsa_sign_cycles: Some(ECDSA_SIGN_CYCLES),
    };

    let chain_id = nat::to_u64(&cfg.chain_id);

    let params = vec![Token::Address(address::to_h160(to)?), value.into_token()];
    let tx_hash = contract
        .signed_call(
            ERC20_TRANSFER_METHOD,
            params,
            options,
            sybil_addr,
            key_info,
            chain_id,
        )
        .await?;

    Ok(format!("0x{}", hex::encode(tx_hash.0)))
}
