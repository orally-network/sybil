pub mod controllers;
pub mod custom_pairs;
pub mod pairs;
pub mod balances;
pub mod transforms;

use ic_cdk::{query, update};

use anyhow::{anyhow, Result};

use ic_utils::{
    api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
    get_information, update_information,
};

use crate::{
    types::rate_data::RateDataLight,
    utils::{get_rate::get_rate, is_pair_exist},
};

#[query]
fn is_pair_exists(pair_id: String) -> bool {
    let (is_exists, _) = is_pair_exist(&pair_id);

    is_exists
}

#[update]
pub async fn get_asset_data_with_proof(pair_id: String) -> Result<RateDataLight, String> {
    _get_asset_data_with_proof(pair_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn _get_asset_data_with_proof(pair_id: String) -> Result<RateDataLight> {
    let (is_exists, metadata) = is_pair_exist(&pair_id);
    if !is_exists {
        return Err(anyhow!("Pair ID does not exist"));
    };

    get_rate(
        metadata.expect("pair metadata should exists after validation"),
        true,
    )
    .await
}

#[update]
pub async fn get_asset_data(pair_id: String) -> Result<RateDataLight, String> {
    _get_asset(pair_id).await.map_err(|e| e.to_string())
}

async fn _get_asset(pair_id: String) -> Result<RateDataLight> {
    let (is_exists, metadata) = is_pair_exist(&pair_id);
    if !is_exists {
        return Err(anyhow!("Pair ID does not exist"));
    };

    let mut rate = get_rate(
        metadata.expect("pair metadata should exists after validation"),
        false,
    )
    .await?;

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
