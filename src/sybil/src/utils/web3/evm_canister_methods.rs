use anyhow::Result;
use candid::Principal;
use cketh_common::eth_rpc::{LogEntry, SendRawTransactionResult};
use ic_cdk::api::call::call_with_payment128;

use ic_web3_rs::{error::TransportError, types::H256, signing::keccak256};
use serde_json::Value;
use crate::types::chains_rpc::{
    GetLogsArgs, EvmRpcConfig, MultiRpcResult, EthCallArgs, BlockTag, ConsensusStrategy,
};

use super::{
    evm_canister_transport_old::{RpcServices, chain_id_to_default_services},
    evm_methods_handlers::*
};


const MAX_CYCLES: u128 = 60_000_000_000;

pub async fn execute_eth_call(
    ic_eth_rpc: Principal,
    chain_id: u64,
    rpc_services: RpcServices,
    call_args: EthCallArgs,
) -> Result<Value, ic_web3_rs::Error> {
    let default_services = chain_id_to_default_services(chain_id);

    let rpc_config = EvmRpcConfig {
        response_size_estimate:None,
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(5), min: 1 }), // TODO remove hardcoded values
    };

    let default_rpc_call = call_with_payment128(
        ic_eth_rpc,
        "eth_call",
        (default_services.clone(), rpc_config.clone(), call_args.clone()),
        MAX_CYCLES,
    )
    .await;

    match default_rpc_call {
        Ok((results,)) => {
            match handle_multi_eth_call_result(results, "with default services") {
                Ok(val) => Ok(val),
                Err(_err) => {
                    let fallback_call = call_with_payment128(
                        ic_eth_rpc,
                        "eth_call",
                        (rpc_services, rpc_config, call_args),
                        MAX_CYCLES,
                    )
                    .await;

                    match fallback_call {
                        Ok((fallback_res,)) => handle_multi_eth_call_result(fallback_res, "with source"),
                        Err((code, msg)) => Err(ic_web3_rs::Error::Transport(
                            TransportError::Message(format!("{code:?}: {msg}"))
                        )),
                    }
                }
            }
        }
        Err((_code, _msg)) => {
            let fallback_call = call_with_payment128(
                ic_eth_rpc,
                "eth_call",
                (rpc_services, rpc_config, call_args),
                MAX_CYCLES,
            )
            .await;

            match fallback_call {
                Ok((fallback_res,)) => handle_multi_eth_call_result(fallback_res, "with source"),
                Err((code2, msg2)) => Err(ic_web3_rs::Error::Transport(
                    TransportError::Message(format!("{code2:?}: {msg2}"))
                )),
            }
        }
    }
}

pub async fn eth_get_logs(
    evm_rpc_canister: Principal,
    chain_id: u64,
    source: RpcServices,
    args: GetLogsArgs,
) -> Result<Value, ic_web3_rs::Error> {
    let default_services = chain_id_to_default_services(chain_id);

    let rpc_config = EvmRpcConfig {
        response_size_estimate:None,
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(5), min: 1 }), // TODO remove hardcoded values
    };

    let (default_result,): (MultiRpcResult<Vec<LogEntry>>,) = call_with_payment128(
        evm_rpc_canister,
        "eth_getLogs",
        (default_services.clone(), rpc_config.clone(), args.clone()),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    match default_result.evaluate() {
        Ok(logs) => {
            Ok(serde_json::to_value(logs).expect("should be able to serialize"))
        }
        Err(_err) => {
            let (source_result,): (MultiRpcResult<Vec<LogEntry>>,) = call_with_payment128(
                evm_rpc_canister,
                "eth_getLogs",
                (source, rpc_config, args),
                MAX_CYCLES,
            )
            .await
            .map_err(|(code, msg)| {
                ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
            })?;

            let logs = source_result.evaluate()?;
            Ok(serde_json::to_value(logs).expect("should be able to serialize"))
        }
    }
}

pub async fn execute_block_number(
    evm_rpc_canister: Principal,
    chain_id: u64,
    source: RpcServices,
    block_tag: BlockTag,
) -> Result<Value, ic_web3_rs::Error> {
    let default_services = chain_id_to_default_services(chain_id);

    let rpc_config = EvmRpcConfig {
        response_size_estimate:None,
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(5), min: 1 }), // TODO remove hardcoded values
    };

    let default_rpc_call = call_with_payment128(
        evm_rpc_canister,
        "eth_getBlockByNumber",
        (default_services.clone(), rpc_config.clone(), block_tag.clone()),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!(
            "IC call error with default services ({code:?}): {msg}"
        )))
    });

    match default_rpc_call {
        Ok((multi_result,)) => {
            match handle_multi_block_by_number_result(multi_result, "with default services") {
                Ok(block_value) => Ok(block_value),
                Err(_) => {
                    let fallback_call = call_with_payment128(
                        evm_rpc_canister,
                        "eth_getBlockByNumber",
                        (source, rpc_config, block_tag),
                        MAX_CYCLES,
                    )
                    .await
                    .map_err(|(code, msg)| {
                        ic_web3_rs::Error::Transport(TransportError::Message(format!(
                            "IC call error with source ({code:?}): {msg}"
                        )))
                    })?;

                    let (fallback_multi_result,) = fallback_call;
                    handle_multi_block_by_number_result(fallback_multi_result, "with source")
                }
            }
        }
        Err(_transport_err) => {
            let fallback_call = call_with_payment128(
                evm_rpc_canister,
                "eth_getBlockByNumber",
                (source, rpc_config, block_tag),
                MAX_CYCLES,
            )
            .await
            .map_err(|(code, msg)| {
                ic_web3_rs::Error::Transport(TransportError::Message(format!(
                    "IC call error with source ({code:?}): {msg}"
                )))
            })?;

            let (fallback_multi_result,) = fallback_call;
            handle_multi_block_by_number_result(fallback_multi_result, "with source")
        }
    }
}


pub async fn send_raw_tx(
    evm_rpc_canister: Principal,
    source: RpcServices,
    raw_tx: Vec<u8>,
) -> Result<Value, ic_web3_rs::Error> {

    let rpc_config = EvmRpcConfig {
        response_size_estimate:None,
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(5), min: 1 }), // TODO remove hardcoded values
    };

    let (results,): (MultiRpcResult<SendRawTransactionResult>,) = call_with_payment128(
        evm_rpc_canister,
        "eth_sendRawTransaction",
        (source, rpc_config, format!("0x{}", hex::encode(raw_tx.clone()))),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    let result = results.evaluate()?;

    if let SendRawTransactionResult::Ok = result {
        Ok(Value::String(format!(
            "{:#?}",
            H256::from_slice(&keccak256(&raw_tx))
        )))
    } else {
        unreachable!("Should be a hash")
    }
}

