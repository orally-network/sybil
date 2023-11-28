use candid::{CandidType, Nat};
use ic_cdk::{query, update};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    log,
    types::{
        data_fetchers::{DataFetcher, DataFetcherError, DataFetchersStorage},
        pairs::Source,
        whitelist::{Whitelist, WhitelistError},
    },
    utils::siwe,
};

#[derive(Error, Debug)]
pub enum DataFetchersError {
    #[error("SIWE error: {0}")]
    Siwe(#[from] siwe::SiweError),
    #[error("Whitelist error: {0}")]
    Whitelist(#[from] WhitelistError),
    #[error("DataFetcher error: {0}")]
    DataFetcher(#[from] DataFetcherError),
    #[error("DataFetcher does not exist")]
    DataFetcherDoesNotExist,
    #[error("Caller is not data fetcher owner")]
    CallerIsNotDataFetcherOwner,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct CreateDataFetcherRequest {
    pub update_freq: Nat,
    pub sources: Vec<Source>,
    pub msg: String,
    pub sig: String,
}

#[update]
pub async fn create_data_fetcher(req: CreateDataFetcherRequest) -> Result<Nat, String> {
    _create_data_fetcher(req)
        .await
        .map_err(|e| format!("create_data_fetcher failed: {}", e))
}

#[inline(always)]
async fn _create_data_fetcher(req: CreateDataFetcherRequest) -> Result<Nat, DataFetchersError> {
    let addr = siwe::recover(&req.msg, &req.sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted.into());
    }

    let data_fetcher = DataFetcher::new(req, &addr);
    data_fetcher.fetch().await?;
    DataFetchersStorage::add(data_fetcher.clone());

    log!(
        "[DATA FETCHERS] data fetcher created. owner: {}, id: {}",
        data_fetcher.owner,
        data_fetcher.id
    );
    Ok(data_fetcher.id)
}

#[update]
pub async fn remove_data_fetcher(id: Nat, msg: String, sig: String) -> Result<(), String> {
    _remove_data_fetcher(id, msg, sig)
        .await
        .map_err(|e| format!("remove_data_fetcher failed: {}", e))
}

#[inline(always)]
pub async fn _remove_data_fetcher(
    id: Nat,
    msg: String,
    sig: String,
) -> Result<(), DataFetchersError> {
    let addr = siwe::recover(&msg, &sig).await?;
    if !Whitelist::contains(&addr) {
        return Err(WhitelistError::AddressNotWhitelisted)?;
    }

    let data_fetcher =
        DataFetchersStorage::get(&id).ok_or(DataFetchersError::DataFetcherDoesNotExist)?;

    if data_fetcher.owner != addr {
        return Err(DataFetchersError::CallerIsNotDataFetcherOwner)?;
    }

    DataFetchersStorage::remove(&id);
    log!(
        "[DATA FETCHERS] data fetcher removed. owner: {}, id: {}",
        addr,
        id
    );
    Ok(())
}

#[update]
pub async fn get_data(id: Nat) -> Result<String, String> {
    _get_data(id)
        .await
        .map_err(|e| format!("get_data failed: {}", e))
}

#[inline(always)]
async fn _get_data(id: Nat) -> Result<String, DataFetchersError> {
    let data_fetcher =
        DataFetchersStorage::get(&id).ok_or(DataFetchersError::DataFetcherDoesNotExist)?;
    Ok(data_fetcher.fetch().await?)
}

#[query]
pub async fn get_data_fetchers(owner: String) -> Vec<DataFetcher> {
    DataFetchersStorage::get_by_owner(&owner)
}
