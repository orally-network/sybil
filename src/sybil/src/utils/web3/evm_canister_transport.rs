#![allow(dead_code)]

use anyhow::Result;
use candid::{CandidType, Principal};
use cketh_common::{
    eth_rpc::{RpcError, SendRawTransactionResult},
    eth_rpc_client::{
        providers::{EthMainnetService, EthSepoliaService, RpcApi, RpcService},
        RpcConfig,
    },
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
    rpc_url: String,
    evm_rpc_canister: Principal,
    max_response_bytes: u64,
}

impl EVMCanisterTransport {
    /// Create new ICEthRpc instance
    pub fn new(rpc_url: String, evm_rpc_canister: Principal) -> Self {
        Self {
            rpc_url,
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

async fn send_raw_tx(
    evm_rpc_canister: Principal,
    source: RpcServices,
    config: Option<RpcConfig>,
    raw_tx: Vec<u8>,
) -> Result<Value, ic_web3_rs::Error> {
    let (result,): (MultiRpcResult<SendRawTransactionResult>,) = call_with_payment128(
        evm_rpc_canister,
        "eth_sendRawTransaction",
        (source, config, format!("0x{}", hex::encode(raw_tx.clone()))),
        MAX_CYCLES,
    )
    .await
    .map_err(|(code, msg)| {
        ic_web3_rs::Error::Transport(TransportError::Message(format!("{:?}: {}", code, msg)))
    })?;

    let MultiRpcResult::Consistent(result) = result else {
        unreachable!("Should be consistent result (or you need to implement handling of inconsistent results, idk, I'm not your mom)")
    };

    result
        .map(|val| {
            if let SendRawTransactionResult::Ok = val {
                Value::String(format!("{:#?}", H256::from_slice(&keccak256(&raw_tx))))
            } else {
                unreachable!("Should be a hash")
            }
        })
        .map_err(|err| ic_web3_rs::Error::InvalidResponse(format!("{:?}", err)))
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
            url: self.rpc_url.clone(),
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
                            services: vec![RpcApi {
                                url: self.rpc_url.clone(),
                                headers: None,
                            }],
                        },
                        None,
                        raw_tx,
                    ))
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
