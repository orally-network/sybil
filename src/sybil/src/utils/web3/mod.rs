#![allow(dead_code)]
use anyhow::Result;
use candid::{CandidType, Nat, Principal};
use ic_cdk::api::management_canister::http_request::{TransformContext, TransformFunc};
use ic_web3_rs::{
    api::Eth,
    contract::{tokens::Tokenizable, Contract, Options},
    ethabi::{Token, TopicFilter},
    ic::KeyInfo,
    transports::{ic_http::CallOptionsBuilder, Batch},
    types::{
        BlockId, BlockNumber, Bytes, CallRequest, FilterBuilder, Log, SignedTransaction,
        Transaction, TransactionId, TransactionReceipt, H160, H256, U256, U64,
    },
    BatchTransport, Transport, Web3,
};
use serde::Deserialize;
use std::{str::FromStr, sync::atomic::AtomicU32, time::Duration};
use thiserror::Error;

use crate::{retry_until_success, types::cache::CacheError};

use self::evm_canister_transport_old::EVMCanisterTransport;

use super::{
    address::{self, AddressError},
    nat, processors, time,
};

pub const SUCCESSFUL_TX_STATUS: u64 = 1;
pub const ECDSA_SIGN_CYCLES: u64 = 26_153_846_153;
pub const ERC20_TRANSFER_METHOD: &str = "transfer";
const TX_WAITING_TIMEOUT: u64 = 60 * 5;
const TX_WAIT_DELAY: Duration = Duration::from_secs(3);

pub mod evm_canister_transport_old;
pub mod promises;
pub mod utils;

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
    #[error("Unable to submit batch: {0}")]
    UnableToSubmitBatch(String),
    #[error("Utils error: {0}")]
    UtilsError(String),
    #[error("From hex error: {0}")]
    FromHexError(String),
    #[error("Cache error: {0}")]
    CacheError(#[from] CacheError),
}

pub struct Web3Instance<T: Transport> {
    w3: Web3<T>,
    pending_requests: AtomicU32,
}

pub fn instance(
    rpc_url: String,
    _evm_rpc_canister: Principal,
    max_response_bytes: Option<u64>,
) -> Web3Instance<impl Transport> {
    // Switch between EVMCanisterTransport(calls go through emv_rpc canister) and ICHttp (calls go straight to the rpc)

    Web3Instance::new(Web3::new(EVMCanisterTransport::new_with_one_rpc(
        rpc_url,
        _evm_rpc_canister,
        max_response_bytes,
    )))

    // Web3Instance::new(Web3::new(ICHttp::new(&rpc_url, None).unwrap()))
}

pub fn batch_instance(
    rpc_url: String,
    evm_rpc_canister: Principal,
    max_response_bytes: Option<u64>,
) -> Web3Instance<Batch<impl BatchTransport>> {
    // Switch between EVMCanisterTransport(calls go through emv_rpc canister) and ICHttp (calls go straight to the rpc)

    Web3Instance::new(Web3::new(Batch::new(
        EVMCanisterTransport::new_with_one_rpc(rpc_url, evm_rpc_canister, max_response_bytes),
    )))

    // Web3Instance::new(Web3::new(Batch::new(ICHttp::new(&rpc_url, None).unwrap())))
}

impl<T: Transport> Web3Instance<T> {
    pub fn new(w3: Web3<T>) -> Self {
        Self {
            w3,
            pending_requests: AtomicU32::new(0),
        }
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
        block_from: Option<u64>,
        block_to: Option<u64>,
        topics0: Option<Vec<H256>>,
        topics1: Option<Vec<H256>>,
        topics2: Option<Vec<H256>>,
        topics3: Option<Vec<H256>>,
        addresses: Option<Vec<H160>>,
    ) -> Result<Vec<Log>, Web3Error> {
        let mut filter_builder = FilterBuilder::default();

        if let Some(from) = block_from {
            filter_builder = filter_builder.from_block(BlockNumber::Number(from.into()));
        }

        if let Some(to) = block_to {
            filter_builder = filter_builder.to_block(BlockNumber::Number(to.into()));
        }

        filter_builder = filter_builder.topic_filter(TopicFilter {
            topic0: if let Some(topics0) = topics0 {
                ic_web3_rs::ethabi::Topic::OneOf(topics0)
            } else {
                ic_web3_rs::ethabi::Topic::Any
            },
            topic1: if let Some(topics1) = topics1 {
                ic_web3_rs::ethabi::Topic::OneOf(topics1)
            } else {
                ic_web3_rs::ethabi::Topic::Any
            },
            topic2: if let Some(topics2) = topics2 {
                ic_web3_rs::ethabi::Topic::OneOf(topics2)
            } else {
                ic_web3_rs::ethabi::Topic::Any
            },
            topic3: if let Some(topics3) = topics3 {
                ic_web3_rs::ethabi::Topic::OneOf(topics3)
            } else {
                ic_web3_rs::ethabi::Topic::Any
            },
        });

        if let Some(addresses) = addresses {
            filter_builder = filter_builder.address(addresses);
        }

        let filter = filter_builder.build();

        let logs = self
            .eth()
            .logs(filter, processors::transform_ctx())
            .await
            .map_err(|err| Web3Error::UnableToGetLogs(err.to_string()))?;

        Ok(logs)
    }

    pub async fn get_tx(&self, tx_hash: H256) -> Result<Transaction, Web3Error> {
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

        retry_until_success!(self
            .eth()
            .transaction_receipt(tx_hash, processors::transform_ctx_tx_with_logs()))
        .map_err(|err| Web3Error::UnableToGetTxReceipt(err.to_string()))?
        .ok_or(Web3Error::TxNotFound)
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

    pub async fn get_block(&self) -> Result<u64, Web3Error> {
        Ok(self
            .eth()
            .block_number(processors::transform_ctx())
            .await
            .map_err(|err| Web3Error::UnableToGetBlockNumber(err.to_string()))?
            .as_u64())
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

    #[allow(clippy::too_many_arguments)]
    pub async fn sign<Tk: Tokenizable + Clone>(
        &self,
        contract: &Contract<T>,
        func: &str,
        params: Vec<Tk>,
        options: Options,
        from: String,
        key_name: String,
        chain_id: u64,
    ) -> Result<SignedTransaction, Web3Error> {
        let signed_call = contract
            .sign(
                func,
                params,
                options,
                from,
                Self::key_info(key_name),
                chain_id,
            )
            .await
            .map_err(|err| Web3Error::UnableToSignContractCall(err.to_string()))?;

        Ok(signed_call)
    }

    pub async fn send_raw_transaction(
        &self,
        signed_call: SignedTransaction,
    ) -> Result<H256, Web3Error> {
        let tx_hash = retry_until_success!(self.eth().send_raw_transaction(
            signed_call.raw_transaction.clone(),
            processors::transform_ctx()
        ))
        .map_err(|err| Web3Error::UnableToExecuteRawTx(err.to_string()))?;

        Ok(tx_hash)
    }

    pub async fn send_raw_transaction_and_wait(
        &self,
        signed_call: SignedTransaction,
    ) -> Result<TransactionReceipt, Web3Error> {
        let tx_hash = self.send_raw_transaction(signed_call).await?;

        self.wait_for_success_confirmation(tx_hash).await
    }

    pub async fn wait_for_success_confirmation(
        &self,
        tx_hash: H256,
    ) -> Result<TransactionReceipt, Web3Error> {
        let receipt = self.wait_for_confirmation(&tx_hash).await?;

        let tx_status = receipt.status.expect("tx should be confirmed").as_u64();

        if tx_status != SUCCESSFUL_TX_STATUS {
            return Err(Web3Error::TxHasFailed);
        }

        Ok(receipt)
    }

    pub async fn wait_for_confirmation(
        &self,
        tx_hash: &H256,
    ) -> Result<TransactionReceipt, Web3Error> {
        let call_opts = CallOptionsBuilder::default()
            .transform(Some(TransformContext {
                function: TransformFunc(candid::Func {
                    principal: ic_cdk::api::id(),
                    method: "transform".into(),
                }),
                context: vec![],
            }))
            .cycles(None)
            .max_resp(None)
            .build()
            .expect("failed to build call options");

        let end_time = time::in_seconds() + TX_WAITING_TIMEOUT;
        while time::in_seconds() < end_time {
            super::sleep(TX_WAIT_DELAY).await;

            let tx_receipt =
                retry_until_success!(self.eth().transaction_receipt(*tx_hash, call_opts.clone()))
                    .map_err(|err| Web3Error::UnableToGetTxReceipt(err.to_string()))?;

            if let Some(tx_receipt) = tx_receipt {
                if tx_receipt.status.is_some() {
                    return Ok(tx_receipt);
                }
            }
        }

        Err(Web3Error::TxTimeout)
    }

    pub async fn get_call_result(
        &self,
        contract: &Contract<T>,
        func: &str,
        params: &[Token],
        from: H160,
        to: Option<H160>,
        block_number: Option<U64>,
    ) -> Result<Vec<Token>, Web3Error> {
        let data = contract
            .abi()
            .function(func)
            .and_then(|f| f.encode_input(params))
            .map_err(|err| Web3Error::UnableToFormCallData(err.to_string()))?;

        let call_request = CallRequest {
            from: Some(from),
            to,
            data: Some(Bytes::from(data)),
            ..Default::default()
        };

        let block_number = block_number.map(|block_number| BlockId::Number(block_number.into()));

        let raw_result = retry_until_success!(self.eth().call(
            call_request.clone(),
            block_number,
            processors::transform_ctx()
        ))
        .map_err(|err| Web3Error::UnableToCallContract(err.to_string()))?;

        let call_result: Vec<Token> = contract
            .abi()
            .function(func)
            .and_then(|f| f.decode_output(&raw_result.0))
            .map_err(|err| Web3Error::UnableToDecodeOutput(err.to_string()))?;

        Ok(call_result)
    }
}
