use anyhow::Result;
use candid::{CandidType, Nat, Principal};
use ic_web3_rs::{
    api::Eth,
    contract::{tokens::Tokenizable, Contract, Options},
    ethabi::{Token, TopicFilter},
    ic::KeyInfo,
    types::{
        BlockNumber, FilterBuilder, Log, Transaction, TransactionId, TransactionReceipt, H160,
        H256, U256,
    },
    Transport, Web3,
};
use serde::Deserialize;
use std::str::FromStr;
use thiserror::Error;

use crate::retry_until_success;

use self::evm_canister_transport::EVMCanisterTransport;

use super::{
    address::{self, AddressError},
    nat, processors,
};

pub const SUCCESSFUL_TX_STATUS: u64 = 1;
pub const ECDSA_SIGN_CYCLES: u64 = 23_000_000_000;
pub const ERC20_TRANSFER_METHOD: &str = "transfer";

mod evm_canister_transport;

#[derive(Error, Debug, CandidType, Deserialize)]
pub enum Web3Error {
    #[error("Address error: {0}")]
    AddressError(#[from] AddressError),
    #[error("Failed to send signed call: {0}")]
    FailedToSendSignedCall(String),
    #[error("Unable to get gas_price: {0}")]
    UnableToGetGasPrice(String),
    #[error("Couldn't convert address to H160: {0}")]
    InvalidAddressFormat(String),
    #[error("Unable to get nonce: {0}")]
    UnableToGetNonce(String),
    #[error("Unable to estimate gas: {0}")]
    UnableToEstimateGas(String),
    #[error("Unable to sign contract call: {0}")]
    UnableToSignContractCall(String),
    #[error("Unable to execute raw transaction: {0}")]
    UnableToExecuteRawTx(String),
    #[error("Unable to get tx receipt: {0}")]
    UnableToGetTxReceipt(String),
    #[error("Tx timeout")]
    TxTimeout,
    #[error("Tx not found")]
    TxNotFound,
    #[error("Tx without receiver")]
    TxWithoutReceiver,
    #[error("Tx has failed")]
    TxHasFailed,
    #[error("Unable to get block number: {0}")]
    UnableToGetBlockNumber(String),
    #[error("Unable to get logs: {0}")]
    UnableToGetLogs(String),
    #[error("Unable to form call data: {0}")]
    UnableToFormCallData(String),
    #[error("Unable to decode output: {0}")]
    UnableToDecodeOutput(String),
    #[error("Unable to call contract: {0}")]
    UnableToCallContract(String),
    #[error("Unable to create contract: {0}")]
    UnableToCreateContract(String),
    #[error("Utils error: {0}")]
    UtilsError(String),
    #[error("From hex error: {0}")]
    FromHexError(String),
}

pub struct Web3Instance<T: Transport> {
    w3: Web3<T>,
}

pub fn instance(rpc_url: String, evm_rpc_canister: Principal) -> Web3Instance<impl Transport> {
    // Switch between EVMCanisterTransport(calls go through emv_rpc canister) and ICHttp (calls go straight to the rpc)

    Web3Instance::new(Web3::new(EVMCanisterTransport::new(
        rpc_url,
        evm_rpc_canister,
    )))

    // Ok(Web3Instance::new(Web3::new(
    //     ICHttp::new(&rpc_url, None).unwrap(),
    // )))
}

impl<T: Transport> Web3Instance<T> {
    pub fn new(w3: Web3<T>) -> Self {
        Self { w3 }
    }

    pub fn eth(&self) -> Eth<T> {
        self.w3.eth()
    }

    #[inline(always)]
    pub fn key_info(key_name: String) -> KeyInfo {
        KeyInfo {
            derivation_path: vec![ic_cdk::id().as_slice().to_vec()],
            key_name,
            ecdsa_sign_cycles: Some(ECDSA_SIGN_CYCLES),
        }
    }

    pub async fn get_logs(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        topic: Option<H256>,
        address: Option<H160>,
        block_hash: Option<H256>,
    ) -> Result<Vec<Log>, Web3Error> {
        let mut filter_builder = FilterBuilder::default();

        if let Some(from) = from {
            filter_builder = filter_builder.from_block(BlockNumber::Number(from.into()));
        }

        if let Some(to) = to {
            filter_builder = filter_builder.from_block(BlockNumber::Number(to.into()));
        }

        if let Some(topic) = topic {
            filter_builder = filter_builder.topic_filter(TopicFilter {
                topic0: ic_web3_rs::ethabi::Topic::This(topic),
                ..Default::default()
            });
        }

        if let Some(address) = address {
            filter_builder = filter_builder.address(vec![address]);
        }

        if let Some(block_hash) = block_hash {
            filter_builder = filter_builder.block_hash(block_hash);
        }

        let logs = self
            .eth()
            .logs(filter_builder.build(), processors::transform_ctx())
            .await
            .map_err(|err| Web3Error::UnableToGetLogs(err.to_string()))?;

        Ok(logs)
    }

    pub async fn get_tx(&self, tx_hash: &str) -> Result<Transaction, Web3Error> {
        let tx_hash =
            H256::from_str(tx_hash).map_err(|err| Web3Error::FromHexError(err.to_string()))?;

        let result = retry_until_success!(self
            .eth()
            .transaction(TransactionId::from(tx_hash), processors::transform_ctx_tx()))
        .map_err(|err| Web3Error::UnableToGetTxReceipt(err.to_string()))?
        .ok_or(Web3Error::TxNotFound)?;

        Ok(result)
    }

    pub async fn get_tx_receipt(&self, tx_hash: &str) -> Result<TransactionReceipt, Web3Error> {
        let tx_hash =
            H256::from_str(tx_hash).map_err(|err| Web3Error::FromHexError(err.to_string()))?;

        Ok(retry_until_success!(self
            .eth()
            .transaction_receipt(tx_hash, processors::transform_ctx_tx_with_logs()))
        .map_err(|err| Web3Error::UnableToGetTxReceipt(err.to_string()))?
        .ok_or(Web3Error::TxNotFound)?)
    }

    pub async fn get_gas_price(&self) -> Result<U256, Web3Error> {
        let gas_price =
            match retry_until_success!(self.eth().gas_price(processors::transform_ctx())) {
                Ok(gas_price) => gas_price,
                Err(e) => Err(Web3Error::UnableToGetGasPrice(e.to_string()))?,
            };

        Ok(gas_price)
    }

    pub async fn get_nonce(&self, account_address: &str) -> Result<U256, Web3Error> {
        let nonce = match retry_until_success!(self.eth().transaction_count(
            H160::from_str(account_address)
                .map_err(|err| Web3Error::InvalidAddressFormat(err.to_string()))?,
            None,
            processors::transform_ctx()
        )) {
            Ok(nonce) => nonce,
            Err(e) => Err(Web3Error::UnableToGetNonce(e.to_string()))?,
        };

        Ok(nonce)
    }

    pub async fn send_erc20(
        &self,
        contract: &Contract<T>,
        value: &Nat,
        to: &str,
        sybil_addr: String,
        key_name: String,
        chain_id: u64,
    ) -> Result<String, Web3Error> {
        let value = nat::to_u256(value);

        let tx_count = self.get_nonce(&sybil_addr).await?;

        let gas_price = self.get_gas_price().await?;
        let options = Options::with(|op| {
            op.nonce = Some(tx_count);
            op.gas_price = Some(gas_price);
        });

        let key_info = Self::key_info(key_name);

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
            .await
            .map_err(|err| Web3Error::FailedToSendSignedCall(err.to_string()))?;

        Ok(format!("0x{}", hex::encode(tx_hash.0)))
    }
}
