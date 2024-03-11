use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{response, HttpRequest, HttpResponse, HTTP_SERVICE};
use crate::types::{feeds::FeedStorage, rate_data::MultipleAssetsDataResult};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAssetDataQueryParams {
    id: String,
}

impl TryFrom<String> for GetAssetDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetMultipleAssetsDataQueryParams {
    ids: Vec<String>,
}

impl TryFrom<String> for GetMultipleAssetsDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
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

pub async fn get_multiple_assets_data_request(req: HttpRequest) -> HttpResponse {
    let resp = _get_multiple_assets_data_request(req, false)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_multiple_assets_data_with_proof_request(req: HttpRequest) -> HttpResponse {
    let resp = _get_multiple_assets_data_request(req, true)
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

pub async fn gather_metrics() -> HttpResponse {
    let data = crate::utils::metrics::gather_metrics();

    response::ok(data)
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

    let rate = FeedStorage::rate(&params.id, with_signature).await?;

    Ok(serde_json::to_vec(&rate)?)
}

#[inline(always)]
async fn _get_multiple_assets_data_request(
    req: HttpRequest,
    with_signature: bool,
) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = GetMultipleAssetsDataQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let mut rate = Vec::with_capacity(params.ids.len());

    for id in params.ids {
        rate.push(FeedStorage::rate(&id, false).await?.data);
    }

    let mut multiple_assets_data_result = MultipleAssetsDataResult {
        data: rate,
        signature: None,
    };

    if with_signature {
        multiple_assets_data_result.sign().await?;
    }

    Ok(serde_json::to_vec(&multiple_assets_data_result)?)
}
