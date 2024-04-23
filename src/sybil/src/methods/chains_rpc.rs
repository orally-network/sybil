use ic_cdk::{query, update};

use crate::{
    types::chains_rpc::{ChainsRPC, ChainsRPCError},
    utils::validate_caller,
};

#[update]
pub fn add_chain_rpc(chain_id: u64, url: String, secret: Option<String>) -> Result<(), String> {
    _add_chain_rpc(chain_id, url, secret).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _add_chain_rpc(
    chain_id: u64,
    url: String,
    secret: Option<String>,
) -> Result<(), ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::add_chain_rpc(chain_id, url, secret);

    Ok(())
}

#[update]
pub fn remove_chain_rpc(chain_id: u64) -> Result<(), String> {
    _remove_chain_rpc(chain_id).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _remove_chain_rpc(chain_id: u64) -> Result<(), ChainsRPCError> {
    validate_caller()?;
    ChainsRPC::remove_chain_rpc(chain_id);

    Ok(())
}

#[query]
pub fn get_chain_rpc(chain_id: u64) -> Result<String, String> {
    _get_chain_rpc(chain_id).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _get_chain_rpc(chain_id: u64) -> Result<String, ChainsRPCError> {
    validate_caller()?;
    Ok(ChainsRPC::get_chain_rpc(chain_id)?)
}

#[query]
pub fn get_all_chains_rpc() -> Result<Vec<(u64, String)>, String> {
    _get_all_chains_rpc().map_err(|e| format!("{e:?}"))
}

#[inline(always)]
pub fn _get_all_chains_rpc() -> Result<Vec<(u64, String)>, ChainsRPCError> {
    validate_caller()?;
    Ok(ChainsRPC::get_all_chains_rpc().into_iter().collect())
}
