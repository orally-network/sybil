use anyhow::Result;

use serde_json::Value;
use crate::types::chains_rpc::{
    MultiEthCallResult, EthCallResult, MultiGetBlockByNumberResult, GetBlockByNumberResult
};

pub fn handle_multi_block_by_number_result(
    multi_result: MultiGetBlockByNumberResult,
    context: &str, 
) -> Result<Value, ic_web3_rs::Error> {
    match multi_result {
        MultiGetBlockByNumberResult::Consistent(get_block_res) => {
            match get_block_res {
                GetBlockByNumberResult::Ok(block) => {
                    let block_number_hex = format!("0x{:x}", block.number.0);
                    Ok(Value::String(block_number_hex))
                }
                GetBlockByNumberResult::Err(rpc_error) => {
                    Err(ic_web3_rs::Error::InvalidResponse(format!(
                        "RPC error {context}: {rpc_error:?}"
                    )))
                }
            }
        }
        MultiGetBlockByNumberResult::Inconsistent(results) => {
            // try to find one block. TODO
            let maybe_ok_block = results.iter().find_map(|(_, res)| match res {
                GetBlockByNumberResult::Ok(block) => Some(block),
                _ => None,
            });

            if let Some(block) = maybe_ok_block {
                let block_number_hex = format!("0x{:x}", block.number.0);
                Ok(Value::String(block_number_hex))
            } else {
                Err(ic_web3_rs::Error::InvalidResponse(format!(
                    "Inconsistent results {context} (no Ok block): {results:?}"
                )))
            }
        }
    }
}

pub fn handle_multi_eth_call_result(
    results: MultiEthCallResult,
    context: &str,
) -> Result<Value, ic_web3_rs::Error> {
    match results {
        MultiEthCallResult::Consistent(EthCallResult::Ok(data)) => {
            Ok(Value::String(data))
        }
        MultiEthCallResult::Consistent(EthCallResult::Err(err)) => {
            Err(ic_web3_rs::Error::InvalidResponse(format!(
                "RPC error {context}: {err:?}"
            )))
        }
        MultiEthCallResult::Inconsistent(results) => {
            let maybe_ok_data = results.iter().find_map(|(_, res)| match res {
                EthCallResult::Ok(data) => Some(data.clone()),
                _ => None,
            });
            if let Some(data) = maybe_ok_data {
                Ok(Value::String(data))
            } else {
                Err(ic_web3_rs::Error::InvalidResponse(format!(
                    "Inconsistent results {context}: {results:?}"
                )))
            }
        }
    }
}