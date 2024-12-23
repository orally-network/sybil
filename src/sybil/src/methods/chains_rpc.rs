use ic_cdk::{query, update};

use crate::{
    types::chains_rpc::{ChainsRPC, ChainsRPCError, RPCConfig, RPCUrl},
    utils::validate_caller,
};

#[update]
pub fn add_chain_rpc(
    chain_id: u64,
    url: String,
    secret: Option<String>,
    config: RPCConfig,
) -> Result<(), String> {
    _add_chain_rpc(chain_id, url, secret, config).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _add_chain_rpc(
    chain_id: u64,
    url: String,
    secret: Option<String>,
    config: RPCConfig,
) -> Result<(), ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::add_chain_rpc(chain_id, url, secret, config);

    Ok(())
}

#[update]
pub fn update_chain_rpc(
    chain_id: u64,
    index: u64,
    url: Option<String>,
    secret: Option<String>,
    config: Option<RPCConfig>,
) -> Result<(), String> {
    _update_chain_rpc(chain_id, index, url, secret, config).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _update_chain_rpc(
    chain_id: u64,
    index: u64,
    url: Option<String>,
    secret: Option<String>,
    config: Option<RPCConfig>,
) -> Result<(), ChainsRPCError> {
    validate_caller()?;

    ChainsRPC::update_chain_rpc(chain_id, index, url, secret, config)
}

#[update]
pub fn remove_chain_rpcs(chain_id: u64) -> Result<(), String> {
    _remove_chain_rpcs(chain_id).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _remove_chain_rpcs(chain_id: u64) -> Result<(), ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::remove_chain_rpcs(chain_id);

    Ok(())
}

#[update]
pub fn remove_chain_rpc_by_index(chain_id: u64, index: usize) -> Result<(), String> {
    _remove_chain_rpc_by_index(chain_id, index).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _remove_chain_rpc_by_index(chain_id: u64, index: usize) -> Result<(), ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::remove_chain_rpc_by_index(chain_id, index);

    Ok(())
}

#[query]
pub fn get_chain_rpc(chain_id: u64) -> Result<String, String> {
    _get_chain_rpc(chain_id).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _get_chain_rpc(chain_id: u64) -> Result<String, ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::get_first_chain_rpc_url(chain_id)
}

#[query]
pub fn get_all_chains_rpc() -> Result<Vec<(u64, Vec<String>)>, String> {
    _get_all_chains_rpc().map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _get_all_chains_rpc() -> Result<Vec<(u64, Vec<String>)>, ChainsRPCError> {
    Ok(ChainsRPC::get_all_chains_rpc().into_iter().collect())
}

#[query]
pub fn get_all_chains_rpc_dev() -> Result<Vec<(u64, Vec<RPCUrl>)>, String> {
    _get_all_chains_rpc_dev().map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _get_all_chains_rpc_dev() -> Result<Vec<(u64, Vec<RPCUrl>)>, ChainsRPCError> {
    validate_caller()?;
    Ok(ChainsRPC::get_all_chains_rpc_dev().into_iter().collect())
}
