use std::collections::HashMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    utils::{validate_caller, CallerError},
    STATE,
};

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct RPCConfig {
    pub num_of_blocks_for_get_dxr_data: u64,
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct RPCUrl {
    pub url: String,
    pub secret: Option<String>,
    pub config: RPCConfig,
}

impl RPCUrl {
    pub fn get_url(&self) -> String {
        if let Some(secret) = &self.secret {
            return self.url.replace("{KEY}", secret);
        }

        self.url.clone()
    }
}

impl RPCUrl {
    pub fn to_string_with_access(&self) -> String {
        if validate_caller().is_ok() {
            self.get_url()
        } else {
            self.url.clone()
        }
    }
}

#[derive(Error, Debug)]
pub enum ChainsRPCError {
    #[error("Chain does not exist")]
    ChainDoesNotExist,
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct ChainsRPC(pub HashMap<u64, Vec<RPCUrl>>);

impl ChainsRPC {
    pub fn add_chain_rpc(chain_id: u64, url: String, secret: Option<String>, config: RPCConfig) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            if state.chains_rpc.0.contains_key(&chain_id) {
                state.chains_rpc.0.get_mut(&chain_id).unwrap().push(RPCUrl {
                    url,
                    secret,
                    config,
                });
            } else {
                state.chains_rpc.0.insert(
                    chain_id,
                    vec![RPCUrl {
                        url,
                        secret,
                        config,
                    }],
                );
            }
        });
    }

    pub fn update_chain_rpc(
        chain_id: u64,
        index: u64,
        url: Option<String>,
        secret: Option<String>,
        config: Option<RPCConfig>,
    ) -> Result<(), ChainsRPCError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(rpc_urls) = state.chains_rpc.0.get_mut(&chain_id) {
                let rpc_url = rpc_urls
                    .get_mut(index as usize)
                    .ok_or(ChainsRPCError::IndexOutOfBounds)?;

                if let Some(url) = url {
                    rpc_url.url = url;
                }

                if let Some(secret) = secret {
                    rpc_url.secret = Some(secret);
                }

                if let Some(config) = config {
                    rpc_url.config = config;
                }

                Ok(())
            } else {
                Err(ChainsRPCError::ChainDoesNotExist)
            }
        })
    }

    pub fn get_first_chain_rpc_url(chain_id: u64) -> Result<String, ChainsRPCError> {
        Ok(STATE
            .with(|state| {
                let state = state.borrow();
                state
                    .chains_rpc
                    .0
                    .get(&chain_id)
                    .cloned()
                    .ok_or(ChainsRPCError::ChainDoesNotExist)
            })?
            .first()
            .unwrap()
            .get_url())
    }

    pub fn get_first_chain_rpc(chain_id: u64) -> Result<RPCUrl, ChainsRPCError> {
        Ok(STATE
            .with(|state| {
                let state = state.borrow();
                state
                    .chains_rpc
                    .0
                    .get(&chain_id)
                    .cloned()
                    .ok_or(ChainsRPCError::ChainDoesNotExist)
            })?
            .first()
            .unwrap()
            .clone())
    }

    pub fn get_chain_rpc(chain_id: u64) -> Result<Vec<String>, ChainsRPCError> {
        Ok(STATE
            .with(|state| {
                let state = state.borrow();
                state.chains_rpc.0.get(&chain_id) // 0 element - provider urls
                    .cloned()
                    .ok_or(ChainsRPCError::ChainDoesNotExist)
            })?
            .into_iter()
            .map(|rpc_url| rpc_url.get_url())
            .collect())
    }

    pub fn get_all_chains_rpc() -> HashMap<u64, Vec<String>> {
        STATE.with(|state| {
            state
                .borrow()
                .chains_rpc
                .0
                .iter()
                .map(|(chain_id, rpc_url)| {
                    (
                        *chain_id,
                        rpc_url
                            .into_iter()
                            .map(|rpc_url| rpc_url.to_string_with_access())
                            .collect(),
                    )
                })
                .collect()
        })
    }

    pub fn get_all_chains_rpc_dev() -> HashMap<u64, Vec<RPCUrl>> {
        STATE.with(|state| state.borrow().chains_rpc.0.clone())
    }

    pub fn remove_chain_rpcs(chain_id: u64) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.chains_rpc.0.remove(&chain_id);
        });
    }

    pub fn remove_chain_rpc_by_index(chain_id: u64, index: usize) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(rpc_urls) = state.chains_rpc.0.get_mut(&chain_id) {
                rpc_urls.remove(index);
            }
        });
    }
}
