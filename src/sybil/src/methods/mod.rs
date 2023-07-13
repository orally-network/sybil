pub mod balances;
pub mod controllers;
pub mod custom_pairs;
pub mod data_fetchers;
pub mod default_pairs;
pub mod transforms;
pub mod whitelist;

use ic_cdk::{query, update};

use thiserror::Error;

use ic_utils::{
    api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
    get_information, update_information,
};

use crate::{
    types::{
        pairs::{PairError, PairsStorage},
        rate_data::RateDataLight,
    },
    utils::canister,
};

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Pair error: {0}")]
    PairError(#[from] PairError),
}

#[query]
fn is_pair_exists(pair_id: String) -> bool {
    PairsStorage::contains(&pair_id)
}

#[query]
fn get_all_pairs() -> Vec<String> {
    PairsStorage::pairs()
}

#[update]
pub async fn get_asset_data_with_proof(pair_id: String) -> Result<RateDataLight, String> {
    _get_asset_data_with_proof(pair_id)
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

pub async fn _get_asset_data_with_proof(pair_id: String) -> Result<RateDataLight, AssetsError> {
    Ok(PairsStorage::rate(&pair_id, true).await?)
}

#[update]
pub async fn get_asset_data(pair_id: String) -> Result<RateDataLight, String> {
    _get_asset_data(pair_id)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

async fn _get_asset_data(pair_id: String) -> Result<RateDataLight, AssetsError> {
    let mut rate = PairsStorage::rate(&pair_id, false).await?;

    rate.signature = None;

    Ok(rate)
}

#[query(name = "getCanistergeekInformation")]
pub async fn get_canistergeek_information(
    request: GetInformationRequest,
) -> GetInformationResponse<'static> {
    get_information(request)
}

#[update(name = "updateCanistergeekInformation")]
pub async fn update_canistergeek_information(request: UpdateInformationRequest) {
    update_information(request);
}

#[update]
pub async fn eth_address() -> Result<String, String> {
    canister::eth_address()
        .await
        .map_err(|e| format!("failed to get eth address: {}", e))
}
