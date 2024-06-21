#![allow(dead_code)]
use ethers_core::abi::{Token, TopicFilter};
use ic_cdk::api::management_canister::http_request::{TransformContext, TransformFunc};
use ic_web3_rs::{
    contract::Contract,
    helpers::CallFuture,
    transports::{ic_http::CallOptionsBuilder, Batch},
    types::{
        BlockId, BlockNumber, Bytes, CallRequest, FilterBuilder, Log, SignedTransaction,
        Transaction, TransactionId, TransactionReceipt, H160, H256, U256, U64,
    },
    BatchTransport, Transport,
};
use serde_json::Value;

use crate::utils::{processors, time};

use super::{Web3Error, Web3Instance, SUCCESSFUL_TX_STATUS, TX_WAITING_TIMEOUT, TX_WAIT_DELAY};

impl<T: BatchTransport> Web3Instance<Batch<T>> {
    pub async fn submit_batch(&self) -> Result<Vec<Result<Value, ic_web3_rs::Error>>, Web3Error> {
        self.w3
            .transport()
            .submit_batch()
            .await
            .map_err(|err| Web3Error::UnableToSubmitBatch(err.to_string()))
    }

    pub fn get_logs_promise(
        &self,
        block_from: Option<u64>,
        block_to: Option<u64>,
        topics0: Option<Vec<H256>>,
        topics1: Option<Vec<H256>>,
        topics2: Option<Vec<H256>>,
        topics3: Option<Vec<H256>>,
        addresses: Option<Vec<H160>>,
    ) -> CallFuture<Vec<Log>, <Batch<T> as Transport>::Out> {
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

        self.eth().logs(filter, processors::transform_ctx())
    }

    pub fn get_tx_promise(
        &self,
        tx_hash: H256,
    ) -> CallFuture<Option<Transaction>, <Batch<T> as Transport>::Out> {
        self.eth()
            .transaction(TransactionId::from(tx_hash), processors::transform_ctx_tx())
    }

    pub fn get_tx_receipt_promise(
        &self,
        tx_hash: H256,
    ) -> CallFuture<Option<TransactionReceipt>, <Batch<T> as Transport>::Out> {
        self.eth()
            .transaction_receipt(tx_hash, processors::transform_ctx_tx_with_logs())
    }

    pub fn get_gas_price_promise(&self) -> CallFuture<U256, <Batch<T> as Transport>::Out> {
        self.eth().gas_price(processors::transform_ctx())
    }

    pub fn get_nonce_promise(
        &self,
        account_address: H160,
    ) -> CallFuture<U256, <Batch<T> as Transport>::Out> {
        self.eth()
            .transaction_count(account_address, None, processors::transform_ctx())
    }

    pub fn send_raw_transaction_promise(
        &self,
        signed_call: SignedTransaction,
    ) -> CallFuture<H256, <Batch<T> as Transport>::Out> {
        self.eth().send_raw_transaction(
            signed_call.raw_transaction.clone(),
            processors::transform_ctx(),
        )
    }

    pub async fn send_raw_transaction_and_wait_promise(
        &self,
        signed_call: SignedTransaction,
    ) -> Result<TransactionReceipt, Web3Error> {
        let tx_hash = self.send_raw_transaction(signed_call);

        self.submit_batch().await.unwrap();

        self.wait_for_success_confirmation_promise(tx_hash.await.unwrap())
            .await
    }

    pub async fn wait_for_success_confirmation_promise(
        &self,
        tx_hash: H256,
    ) -> Result<TransactionReceipt, Web3Error> {
        let receipt = self.wait_for_confirmation_promise(&tx_hash).await?;

        let tx_status = receipt.status.expect("tx should be confirmed").as_u64();

        if tx_status != SUCCESSFUL_TX_STATUS {
            return Err(Web3Error::TxHasFailed);
        }

        Ok(receipt)
    }

    pub async fn wait_for_confirmation_promise(
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
            crate::utils::sleep(TX_WAIT_DELAY).await;

            let tx_receipt = self.eth().transaction_receipt(*tx_hash, call_opts.clone());

            self.submit_batch().await.unwrap();

            if let Some(tx_receipt) = tx_receipt.await.unwrap() {
                if tx_receipt.status.is_some() {
                    return Ok(tx_receipt);
                }
            }
        }

        Err(Web3Error::TxTimeout)
    }

    pub async fn get_call_result_promise<Tr: Transport>(
        &self,
        contract: &Contract<Tr>,
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

        let raw_result = self.eth().call(
            call_request.clone(),
            block_number,
            processors::transform_ctx(),
        );

        self.submit_batch()
            .await
            .map_err(|err| Web3Error::UnableToSubmitBatch(err.to_string()))?;

        let raw_result = raw_result
            .await
            .map_err(|err| Web3Error::UnableToCallContract(err.to_string()))?;

        let call_result: Vec<Token> = contract
            .abi()
            .function(func)
            .and_then(|f| f.decode_output(&raw_result.0))
            .map_err(|err| Web3Error::UnableToDecodeOutput(err.to_string()))?;

        Ok(call_result)
    }
}
