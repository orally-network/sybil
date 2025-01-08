use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::Result;
use candid::{CandidType, Principal};
use cketh_common::{
    eth_rpc_client::providers::{EthMainnetService, EthSepoliaService, RpcApi, RpcService},
    numeric::BlockNumber,
};
use ic_cdk::api::call::call_with_payment128;
use ic_web3_rs::{
    error::TransportError, futures::future::BoxFuture, helpers,
    transports::ic_http::CallOptions, BatchTransport, RequestId, Transport,
};
use jsonrpc_core::{Call, Output, Params, Request};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use super::{
    evm_canister_methods::*,
    utils::convert_to_call_args,
};

use crate::{log, retry_until_success};
use crate::types::chains_rpc::{
    GetLogsRpcConfig, ConsensusStrategy, GetLogsArgs, BlockTag, MultiRpcResult,
};

const MAX_CYCLES: u128 = 60_000_000_000;
const DEFAULT_MAX_RESPONSE_BYTES: u64 = 10_000;

/// ICEthRpc deals with the JSON-RPC canister nametd "ic-eth-rpc" which is deployed on the IC.
#[derive(Clone, Debug)]
pub struct EVMCanisterTransport {
    chain_id: u64,
    pub rpcs_url: Option<Vec<String>>,
    evm_rpc_canister: Principal,
    max_response_bytes: u64,
    id: Arc<AtomicUsize>,
}

impl EVMCanisterTransport {
    /// Create new ICEthRpc instance
    pub fn new_with_one_rpc(
        chain_id: u64,
        rpc_url: String,
        evm_rpc_canister: Principal,
        max_response_bytes: Option<u64>,
    ) -> Self {
        Self {
            chain_id,
            rpcs_url: Some(vec![rpc_url]),
            evm_rpc_canister,
            max_response_bytes: max_response_bytes.unwrap_or(DEFAULT_MAX_RESPONSE_BYTES),
            id: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn new(chain_id: u64, rpcs_url: Option<Vec<String>>, evm_rpc_canister: Principal) -> Self {
        Self {
            chain_id,
            rpcs_url,
            evm_rpc_canister,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            id: Arc::new(AtomicUsize::new(0)),
        }
    }

    // we return constant id because ic_eth_rpc doesn't use it
    pub fn next_id(&self) -> RequestId {
        self.id.fetch_add(1, Ordering::AcqRel)
    }
}

async fn execute_canister_request_batch<T: DeserializeOwned>(
    ic_eth_rpc: Principal,
    service: RpcService,
    json_rpc_payload: String,
    max_response_bytes: u64,
) -> Result<T, ic_web3_rs::Error> {
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

    let output: T = serde_json::from_str(&result).unwrap();

    Ok(output)
}

async fn execute_canister_request(
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

#[derive(Clone, CandidType, Deserialize, Debug)]
pub enum RpcServices {
    EthMainnet(Option<Vec<EthMainnetService>>),
    EthSepolia(Option<Vec<EthSepoliaService>>),
    Custom {
        #[serde(rename = "chainId")]
        chain_id: u64,
        services: Vec<RpcApi>,
    },
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
                        Err(ic_web3_rs::Error::InvalidResponse(format!(
                            "All results are errors: {:?}",
                            results
                        )))
                    }
                }
            }
        }
    }
}

fn id_of_output(output: &Output) -> Result<RequestId, ic_web3_rs::Error> {
    let id = match output {
        Output::Success(success) => &success.id,
        Output::Failure(failure) => &failure.id,
    };
    match id {
        jsonrpc_core::Id::Num(num) => Ok(*num as RequestId),
        _ => Err(ic_web3_rs::Error::InvalidResponse(
            "response id is not u64".to_string(),
        )),
    }
}

// According to the jsonrpc specification batch responses can be returned in any order so we need to
// restore the intended order.
fn handle_batch_response(
    ids: &[RequestId],
    outputs: Vec<Output>,
) -> Result<Vec<Result<Value, ic_web3_rs::Error>>, ic_web3_rs::Error> {
    if ids.len() != outputs.len() {
        return Err(ic_web3_rs::Error::InvalidResponse(
            "unexpected number of responses".to_string(),
        ));
    }
    let mut outputs = outputs
        .into_iter()
        .map(|output| {
            Ok((
                id_of_output(&output)?,
                helpers::to_result_from_output(output),
            ))
        })
        .collect::<Result<HashMap<_, _>>>()
        .unwrap();

    ids.iter()
        .map(|id| {
            outputs.remove(id).ok_or_else(|| {
                ic_web3_rs::Error::InvalidResponse(format!("batch response is missing id {}", id))
            })
        })
        .collect()
}

// TODO implement traits
fn map_chain_id_to_rpc_services(chain_id: u64) -> RpcServices {
    match chain_id {
        1 => RpcServices::EthMainnet(None),
        5 => RpcServices::EthSepolia(None),
        _ => panic!("Unsupported chain_id: {}", chain_id),
    }
}

fn map_chain_id_to_rpc_service(chain_id: u64) -> RpcService {
    match chain_id {
        1 => RpcService::EthMainnet(EthMainnetService::Ankr),
        11155111 => RpcService::EthSepolia(EthSepoliaService::Ankr),
        _ => panic!("Unsupported chain_id: {}", chain_id),
    }
}

impl BatchTransport for EVMCanisterTransport {
    type Batch =
        BoxFuture<'static, Result<Vec<Result<Value, ic_web3_rs::Error>>, ic_web3_rs::Error>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, Call)>,
    {
        let (ids, calls): (Vec<_>, Vec<_>) = requests.into_iter().unzip();

        let json_rpc_payload = serde_json::to_string(&Request::Batch(calls.clone())).unwrap();

        let service = RpcService::Custom(RpcApi {
            url: self.rpcs_url.clone().unwrap().first().unwrap().clone(), // TODO remove unwrap
            headers: None,
        });
        
        let rpcs_url = self.rpcs_url.clone();
        let evm_rpc_canister = self.evm_rpc_canister;
        let max_response_bytes = self.max_response_bytes;
        let chain_id = self.chain_id;

        Box::pin(async move {
            let mut results = Vec::new();

            for call in calls {
                match call {
                    Call::MethodCall(ref method_call) if method_call.method == "eth_call" => {
                        let Params::Array(ref arr) = method_call.params else {
                            unreachable!()
                        };

                        let call_args = match convert_to_call_args(arr) {
                            Ok(args) => args,
                            Err(e) => {
                                results.push(Err(e));
                                continue;
                            }
                        };
                        let eth_call_result = execute_eth_call(
                            evm_rpc_canister,
                            RpcServices::Custom {
                                chain_id, 
                                services: rpcs_url
                                    .clone()
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|url| RpcApi {
                                        url: url.clone(),
                                        headers: None,
                                    })
                                    .collect(),
                            },
                            call_args,
                            None,
                        )
                        .await;

                        results.push(eth_call_result);
                    }
                    Call::MethodCall(ref method_call) if method_call.method == "eth_blockNumber" => {
                        let eth_block_number_result = execute_block_number(
                            evm_rpc_canister,
                            RpcServices::Custom {
                                chain_id, 
                                services: rpcs_url
                                    .clone()
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|url| RpcApi {
                                        url: url.clone(),
                                        headers: None,
                                    })
                                    .collect(),
                            },
                            BlockTag::Latest,
                            None,
                        )
                        .await;

                        results.push(eth_block_number_result);
                    }
                    _ => {
                        let outputs: Result<Vec<Output>, ic_web3_rs::Error> =
                            retry_until_success!(execute_canister_request_batch(
                                evm_rpc_canister,
                                service.clone(),
                                json_rpc_payload.clone(),
                                max_response_bytes,
                            ));

                        match outputs {
                            Ok(outs) => {
                                let batch_response = handle_batch_response(&ids, outs);
                                results.extend(batch_response.unwrap_or_default());
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            Ok(results)
        })
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
        let get_logs_config = GetLogsRpcConfig {
            response_size_estimate:None,
            response_consensus: Some(ConsensusStrategy::Threshold { total: Some(5), min: 1 }), // TODO remove hardcoded values
        };

        // if rpcs_url is not set, we use the default chain rpc for the chain_id
        let source = match self.rpcs_url {
            Some(ref rpc_urls) => RpcServices::Custom {
                chain_id: self.chain_id,
                services: rpc_urls
                    .iter()
                    .map(|url| RpcApi {
                        url: url.clone(),
                        headers: None,
                    })
                    .collect(),
            },
            None => map_chain_id_to_rpc_services(self.chain_id),
        };

        let json_rpc_payload = serde_json::to_string(&Request::Single(call.clone())).unwrap();

        log!("json_rpc_payload: {}", json_rpc_payload);
        log!(
            "json_rpc_payload_batch: {}",
            serde_json::to_string(&Request::Batch(vec![call.clone()])).unwrap()
        );

        let ic_eth_rpc = self.evm_rpc_canister;
        let max_response_bytes = self.max_response_bytes;

        let evm_rpc_request_service = map_chain_id_to_rpc_service(self.chain_id);

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
                        source,
                        Some(get_logs_config),
                        raw_tx,
                    ))
                }
                "eth_getLogs" => {
                    let Params::Array(ref arr) = method_call.params else {
                        unreachable!()
                    };

                    let value = arr[0].clone();

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
                    Box::pin(async move { eth_get_logs(ic_eth_rpc, source, Some(get_logs_config), args).await })
                }
                _ => Box::pin(async move {
                    execute_canister_request(ic_eth_rpc, evm_rpc_request_service, json_rpc_payload, max_response_bytes)
                        .await
                }),
            },
            _ => Box::pin(async move {
                execute_canister_request(ic_eth_rpc, evm_rpc_request_service, json_rpc_payload, max_response_bytes)
                    .await
            }),
        }
    }

    fn set_max_response_bytes(&mut self, v: u64) {
        self.max_response_bytes = v;
    }
}
