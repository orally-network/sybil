use std::{collections::HashMap, fmt::Display};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    utils::{validate_caller, CallerError},
    STATE,
};

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct RPCUrl {
    pub url: String,
    pub secret: Option<String>,
}

impl RPCUrl {
    pub fn get_url(&self) -> String {
        if let Some(secret) = &self.secret {
            return self.url.replace("{KEY}", secret);
        }

        self.url.clone()
    }
}

impl Display for RPCUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if validate_caller().is_ok() {
            write!(f, "{}", self.get_url())
        } else {
            write!(f, "{}", self.url)
        }
    }
}

#[derive(Error, Debug)]
pub enum ChainsRPCError {
    #[error("Chain does not exist")]
    ChainDoesNotExist,
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct ChainsRPC(HashMap<u64, RPCUrl>);

impl ChainsRPC {
    pub fn add_chain_rpc(chain_id: u64, url: String, secret: Option<String>) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.chains_rpc.0.insert(chain_id, RPCUrl { url, secret });
        });
    }

    pub fn get_chain_rpc(chain_id: u64) -> Result<String, ChainsRPCError> {
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
            .get_url())
    }

    pub fn get_all_chains_rpc() -> HashMap<u64, String> {
        STATE.with(|state| {
            state
                .borrow()
                .chains_rpc
                .0
                .iter()
                .map(|(chain_id, rpc_url)| (*chain_id, rpc_url.to_string()))
                .collect()
        })
    }

    pub fn remove_chain_rpc(chain_id: u64) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.chains_rpc.0.remove(&chain_id);
        });
    }
}
