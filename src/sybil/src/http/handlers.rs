use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{response, HttpRequest, HttpResponse, HTTP_SERVICE};
use crate::{types::pairs::PairsStorage, utils::validation};
use crate::{types::{service, auth_data}};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAssetDataQueryParams {
    #[validate(regex = "validation::PAIR_ID_REGEX")]
    pair_id: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAuthDataQueryParams {
    service: String,
    access_token: String,
    user_id: String,
}

impl TryFrom<String> for GetAssetDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

pub async fn verify_access_token(req: HttpRequest) -> HttpResponse {
    let resp = _verify_access_token(req).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_asset_data_request(req: HttpRequest) -> HttpResponse {
    let resp = _get_asset_data_request(req, false)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_asset_data_with_proof_request(req: HttpRequest) -> HttpResponse {
    let resp = _get_asset_data_request(req, true)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _get_asset_data_request(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");
    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = GetAssetDataQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let rate = PairsStorage::rate(&params.pair_id, with_signature).await?;

    Ok(serde_json::to_vec(&rate)?)
}

#[inline(always)]
pub async fn _verify_access_token(req: HttpRequest) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");
    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;
    
    let params = GetAssetDataQueryParams::try_from(query.to_string())?;
    params.validate()?;
    
    
    
    Ok(serde_json::to_vec(&rate)?)
}
