use anyhow::Result;
use candid::Principal;
use cketh_common::{
    eth_rpc::{LogEntry, SendRawTransactionResult},
    eth_rpc_client::RpcConfig,
};
use ic_cdk::api::call::call_with_payment128;

use ic_web3_rs::{error::TransportError, types::H256, signing::keccak256};
use serde_json::Value;
use crate::types::chains_rpc::{
    GetLogsArgs, GetLogsRpcConfig, MultiRpcResult, MultiEthCallResult, EthCallArgs, 
    EthCallResult, MultiGetBlockByNumberResult, GetBlockByNumberResult, BlockTag
};
use super::evm_canister_transport_old::RpcServices;

const MAX_CYCLES: u128 = 60_000_000_000;

pub async fn execute_eth_call(
    ic_eth_rpc: Principal,
    rpc_services: RpcServices,
    call_args: EthCallArgs,
    config: Option<RpcConfig>,
) -> Result<Value, ic_web3_rs::Error> {
    let (results,): (MultiEthCallResult,) = call_with_payment128(
        ic_eth_rpc,
        "eth_call",
        (rpc_services, config, call_args),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    match results {
        MultiEthCallResult::Consistent(EthCallResult::Ok(data)) => {
            Ok(serde_json::Value::String(data))
        }
        MultiEthCallResult::Consistent(EthCallResult::Err(err)) => Err(ic_web3_rs::Error::InvalidResponse(format!(
            "RPC error: {:?}",
            err
        ))),
        MultiEthCallResult::Inconsistent(results) => Err(ic_web3_rs::Error::InvalidResponse(format!(
            "Inconsistent results: {:?}",
            results
        ))),
    }
}

pub async fn eth_get_logs(
    evm_rpc_canister: Principal,
    source: RpcServices,
    config: Option<GetLogsRpcConfig>,
    args: GetLogsArgs,
) -> Result<Value, ic_web3_rs::Error> {
    let (results,): (MultiRpcResult<Vec<LogEntry>>,) = call_with_payment128(
        evm_rpc_canister,
        "eth_getLogs",
        (source, config, args),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    let result = results.evaluate()?;
    Ok(serde_json::to_value(result).expect("should be able to serialize"))
}

pub async fn execute_block_number(
    evm_rpc_canister: Principal,
    source: RpcServices,
    block_tag: BlockTag,
    config: Option<RpcConfig>,
) -> Result<Value, ic_web3_rs::Error> {
    let (result,): (MultiGetBlockByNumberResult,) = call_with_payment128(
        evm_rpc_canister,
        "eth_getBlockByNumber",
        (source, config, block_tag),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!(
            "IC call error ({:?}): {}",
            code, msg
        )))
    })?;

    match result {
        MultiGetBlockByNumberResult::Consistent(get_block_res) => match get_block_res {
            GetBlockByNumberResult::Ok(block) => {
                let block_number_hex = format!("0x{:x}", block.number.0);
                Ok(serde_json::Value::String(block_number_hex))
            }
            GetBlockByNumberResult::Err(rpc_error) => Err(ic_web3_rs::Error::InvalidResponse(
                format!("RPC error: {:?}", rpc_error),
            )),
        },
        MultiGetBlockByNumberResult::Inconsistent(results) => {
            let maybe_ok_block = results.iter().find_map(|(_, res)| match res {
                GetBlockByNumberResult::Ok(block) => Some(block),
                _ => None,
            });

            if let Some(block) = maybe_ok_block {
                let block_number_hex = format!("0x{:x}", block.number.0);
                Ok(serde_json::Value::String(block_number_hex))
            } else {
                Err(ic_web3_rs::Error::InvalidResponse(format!(
                    "Inconsistent results (no Ok block): {:?}",
                    results
                )))
            }
        }
    }
}

pub async fn send_raw_tx(
    evm_rpc_canister: Principal,
    source: RpcServices,
    config: Option<GetLogsRpcConfig>,
    raw_tx: Vec<u8>,
) -> Result<Value, ic_web3_rs::Error> {
    let (results,): (MultiRpcResult<SendRawTransactionResult>,) = call_with_payment128(
        evm_rpc_canister,
        "eth_sendRawTransaction",
        (source, config, format!("0x{}", hex::encode(raw_tx.clone()))),
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

