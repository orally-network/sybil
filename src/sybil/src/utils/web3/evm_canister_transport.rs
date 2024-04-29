#![allow(dead_code)]

use std::fmt::Debug;

use anyhow::Result;
use candid::{CandidType, Principal};
use cketh_common::{
    eth_rpc::{LogEntry, RpcError, SendRawTransactionResult},
    eth_rpc_client::{
        providers::{EthMainnetService, EthSepoliaService, RpcApi, RpcService},
        RpcConfig,
    },
    numeric::BlockNumber,
};
use ic_cdk::api::call::call_with_payment128;
use ic_web3_rs::{
    error::TransportError, futures::future::BoxFuture, helpers, signing::keccak256,
    transports::ic_http::CallOptions, types::H256, RequestId, Transport,
};
use jsonrpc_core::{Call, Output, Params, Request};
use serde::Deserialize;
use serde_json::Value;

const MAX_CYCLES: u128 = 60_000_000_000;
const DEFAULT_MAX_RESPONSE_BYTES: u64 = 100000;

/// ICEthRpc deals with the JSON-RPC canister nametd "ic-eth-rpc" which is deployed on the IC.
#[derive(Clone, Debug)]
pub struct EVMCanisterTransport {
    rpcs_url: Vec<String>,
    evm_rpc_canister: Principal,
    max_response_bytes: u64,
}

impl EVMCanisterTransport {
    /// Create new ICEthRpc instance
    pub fn new_with_one_rpc(rpc_url: String, evm_rpc_canister: Principal) -> Self {
        Self {
            rpcs_url: vec![rpc_url],
            evm_rpc_canister,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn new(rpcs_url: Vec<String>, evm_rpc_canister: Principal) -> Self {
        Self {
            rpcs_url,
            evm_rpc_canister,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    // we return constant id because ic_eth_rpc doesn't use it
    pub fn next_id(&self) -> RequestId {
        1
    }
}

async fn execute_canister_call(
    ic_eth_rpc: Principal,
    service: RpcService,
    json_rpc_payload: String,
    max_response_bytes: u64,
) -> Result<Value, ic_web3_rs::Error> {
    let (result,): (Result<String, cketh_common::eth_rpc::RpcError>,) = call_with_payment128(
        ic_eth_rpc,
        "request",
        (service, json_rpc_payload, max_response_bytes),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    let result = result.map_err(|err| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!(
            "Error in ic_eth_rpc: {:?}",
            err
        )))
    })?;

    let output: Output = serde_json::from_str(&result).unwrap();

    match output {
        Output::Success(success) => Ok(success.result),
        Output::Failure(failure) => Err(ic_web3_rs::Error::Transport(TransportError::Message(
            failure.error.message,
        ))),
    }
}

#[derive(Clone, CandidType, Deserialize)]
pub enum RpcServices {
    EthMainnet(Option<Vec<EthMainnetService>>),
    EthSepolia(Option<Vec<EthSepoliaService>>),
    Custom {
        #[serde(rename = "chainId")]
        chain_id: u64,
        services: Vec<RpcApi>,
    },
}

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Clone, Debug, Eq, PartialEq, CandidType, Deserialize)]
pub enum MultiRpcResult<T> {
    Consistent(RpcResult<T>),
    Inconsistent(Vec<(RpcService, RpcResult<T>)>),
}

impl<T: Debug + Clone> MultiRpcResult<T> {
    pub fn evaluate(&self) -> Result<T, ic_web3_rs::Error> {
        match self {
            MultiRpcResult::Consistent(result) => result
                .clone()
                .map_err(|err| ic_web3_rs::Error::InvalidResponse(format!("{:?}", err))),
            MultiRpcResult::Inconsistent(results) => {
                let result =
                    results.iter().find_map(
                        |(_, result)| {
                            if result.is_ok() {
                                Some(result)
                            } else {
                                None
                            }
                        },
                    );

                match result {
                    Some(result) => result
                        .clone()
                        .map_err(|err| ic_web3_rs::Error::InvalidResponse(format!("{:?}", err))),
                    None => {
                        return Err(ic_web3_rs::Error::InvalidResponse(format!(
                            "All results are errors: {:?}",
                            results
                        )))
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Default)]
pub enum BlockTag {
    #[default]
    Latest,
    Finalized,
    Safe,
    Earliest,
    Pending,
    Number(BlockNumber),
}

#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize)]
pub struct GetLogsArgs {
    #[serde(rename = "fromBlock")]
    pub from_block: Option<BlockTag>,
    #[serde(rename = "toBlock")]
    pub to_block: Option<BlockTag>,
    pub addresses: Vec<String>,
    pub topics: Option<Vec<Vec<String>>>,
}

async fn eth_get_logs(
    evm_rpc_canister: Principal,
    source: RpcServices,
    config: Option<RpcConfig>,
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

async fn send_raw_tx(
    evm_rpc_canister: Principal,
    source: RpcServices,
    config: Option<RpcConfig>,
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

impl Transport for EVMCanisterTransport {
    type Out = BoxFuture<'static, Result<Value, ic_web3_rs::Error>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        let id = self.next_id();
        let request = helpers::build_request(id, method, params);
        (id, request)
    }

    fn send(&self, _: RequestId, call: Call, _: CallOptions) -> Self::Out {
        let service: RpcService = RpcService::Custom(RpcApi {
            url: self.rpcs_url.get(0).unwrap().clone(),
            headers: None,
        });

        let json_rpc_payload = serde_json::to_string(&Request::Single(call.clone())).unwrap();

        let ic_eth_rpc = self.evm_rpc_canister;
        let max_response_bytes = self.max_response_bytes;

        match call {
            Call::MethodCall(method_call) => match method_call.method.as_str() {
                "eth_sendRawTransaction" => {
                    let Params::Array(ref arr) = method_call.params else {
                        unreachable!()
                    };

                    let raw_tx = hex::decode(&arr[0].as_str().unwrap()[2..])
                        .expect("should be able to parse");

                    Box::pin(send_raw_tx(
                        ic_eth_rpc,
                        RpcServices::Custom {
                            chain_id: 5,
                            services: self
                                .rpcs_url
                                .iter()
                                .map(|url| RpcApi {
                                    url: url.clone(),
                                    headers: None,
                                })
                                .collect(),
                        },
                        None,
                        raw_tx,
                    ))
                }
                "eth_getLogs" => {
                    let Params::Array(ref arr) = method_call.params else {
                        unreachable!()
                    };

                    let value = arr[0].clone();

                    let services = RpcServices::Custom {
                        chain_id: 0,
                        services: self
                            .rpcs_url
                            .iter()
                            .map(|url| RpcApi {
                                url: url.clone(),
                                headers: None,
                            })
                            .collect(),
                    };

                    let from_block = value.get("fromBlock").map(|v| {
                        if v.is_string() {
                            BlockTag::Number(
                                BlockNumber::from_str_hex(v.as_str().unwrap()).unwrap(),
                            )
                        } else {
                            unreachable!();
                        }
                    });

                    let to_block = value.get("toBlock").map(|v| {
                        if v.is_string() {
                            BlockTag::Number(
                                BlockNumber::from_str_hex(v.as_str().unwrap()).unwrap(),
                            )
                        } else {
                            unreachable!();
                        }
                    });

                    let addresses = value
                        .get("address")
                        .map(|v| {
                            if v.is_array() {
                                v.as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|v| v.as_str().unwrap().to_string())
                                    .collect()
                            } else {
                                vec![v.as_str().unwrap().to_string()]
                            }
                        })
                        .unwrap_or_default();

                    let topics = value.get("topics").map(|v| {
                        if v.is_array() {
                            v.as_array()
                                .unwrap()
                                .iter()
                                .map(|v| {
                                    if v.is_array() {
                                        v.as_array()
                                            .unwrap()
                                            .iter()
                                            .map(|v| v.as_str().unwrap().to_string())
                                            .collect()
                                    } else {
                                        vec![v.as_str().unwrap().to_string()]
                                    }
                                })
                                .collect()
                        } else {
                            unreachable!();
                        }
                    });

                    let args = GetLogsArgs {
                        from_block,
                        to_block,
                        addresses,
                        topics,
                    };

                    Box::pin(async move { eth_get_logs(ic_eth_rpc, services, None, args).await })
                    // Box::pin(async move {
                    //     execute_canister_call(
                    //         ic_eth_rpc,
                    //         service,
                    //         json_rpc_payload,
                    //         max_response_bytes,
                    //     )
                    //     .await
                    // })
                }
                _ => Box::pin(async move {
                    execute_canister_call(ic_eth_rpc, service, json_rpc_payload, max_response_bytes)
                        .await
                }),
            },
            _ => Box::pin(async move {
                execute_canister_call(ic_eth_rpc, service, json_rpc_payload, max_response_bytes)
                    .await
            }),
        }
    }

    fn set_max_response_bytes(&mut self, v: u64) {
        self.max_response_bytes = v;
    }
}
