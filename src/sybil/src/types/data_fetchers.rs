use std::collections::HashMap;

use ic_cdk::export::{
    candid::{CandidType, Nat},
    serde::{Deserialize, Serialize},
};

use ic_web3_rs::futures::future::join_all;
use thiserror::Error;

use crate::{
    methods::data_fetchers::CreateDataFetcherRequest,
    types::pairs::Source,
    utils::{canister, nat, vec},
    STATE,
};

use super::{
    balances::{BalanceError, Balances},
    state, Address,
};

#[derive(Error, Debug)]
pub enum DataFetcherError {
    #[error("DataFetcher not found")]
    DataFetcherNotFound,
    #[error("Canister error: {0}")]
    Canister(#[from] canister::CanisterError),
    #[error("Balance error: {0}")]
    Balance(#[from] BalanceError),
    #[error("No value got from sources")]
    NoValueGotFromSources,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFetcher {
    pub id: Nat,
    pub update_freq: Nat,
    pub owner: Address,
    pub sources: Vec<Source>,
}

impl DataFetcher {
    pub async fn fetch(&self) -> Result<String, DataFetcherError> {
        let canister_addr = canister::eth_address().await?;

        let bytes = Nat::from(
            self.sources
                .iter()
                .fold(0, |v, source| v + source.expected_bytes),
        );
        let fee_per_byte = state::get_cfg().balances_cfg.fee_per_byte;
        let fee = fee_per_byte * bytes;

        if !Balances::is_sufficient(&self.owner, &fee)? {
            return Err(BalanceError::InsufficientBalance)?;
        };

        let update_freq = nat::to_u64(&self.update_freq);

        let futures = self
            .sources
            .iter()
            .map(|source| source.data(update_freq))
            .collect::<Vec<_>>();

        let (mut results, _) = join_all(futures)
            .await
            .iter()
            .filter_map(|res| res.as_ref().ok().cloned())
            .unzip::<_, _, Vec<_>, Vec<_>>();

        Balances::reduce_amount(&self.owner, &fee)?;
        Balances::add_amount(&canister_addr, &fee)?;

        results.sort();

        vec::find_most_frequent_value(&results).ok_or(DataFetcherError::NoValueGotFromSources)
    }

    pub fn new(req: CreateDataFetcherRequest, owner: &Address) -> DataFetcher {
        DataFetcher {
            id: DataFethcersIndexer::next(),
            update_freq: req.update_freq,
            owner: owner.clone(),
            sources: req.sources,
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFetchersStorage(HashMap<Nat, DataFetcher>);

impl DataFetchersStorage {
    pub fn add(data_fetcher: DataFetcher) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state
                .data_fetchers
                .0
                .insert(data_fetcher.id.clone(), data_fetcher.clone());
        })
    }

    pub fn remove(id: &Nat) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.data_fetchers.0.remove(id);
        })
    }

    pub fn get(id: &Nat) -> Option<DataFetcher> {
        STATE.with(|state| {
            let state = state.borrow();
            state.data_fetchers.0.get(id).cloned()
        })
    }

    pub fn get_by_owner(owner: &Address) -> Vec<DataFetcher> {
        STATE.with(|state| {
            let state = state.borrow();
            state
                .data_fetchers
                .0
                .values()
                .filter(|df| df.owner == *owner)
                .cloned()
                .collect()
        })
    }

    pub async fn fetch(id: &Nat) -> Result<String, DataFetcherError> {
        let data_fetcher = Self::get(id).ok_or(DataFetcherError::DataFetcherNotFound)?;
        data_fetcher.fetch().await
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone, Default)]
pub struct DataFethcersIndexer(Nat);

impl DataFethcersIndexer {
    pub fn next() -> Nat {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.data_fetchers_indexer.0 += 1;
            state.data_fetchers_indexer.0.clone()
        })
    }
}
