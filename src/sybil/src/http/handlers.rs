use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{response, utils::resolve_payer, HttpRequest, HttpResponse, HTTP_SERVICE};
use crate::methods::{_get_asset_data, _get_multiple_assets_data};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAssetDataQueryParams {
    id: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
    bytes: Option<bool>,
}

impl TryFrom<String> for GetAssetDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetMultipleAssetsDataQueryParams {
    ids: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
    bytes: Option<bool>,
}

impl TryFrom<String> for GetMultipleAssetsDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetXRCDataQueryParams {
    id: String,
    api_key: Option<String>,
    msg: Option<String>,
    sig: Option<String>,
    bytes: Option<bool>,
}

impl TryFrom<String> for GetXRCDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct ReadLogsQueryParams {
    chain_id: u64,
    block_from: Option<u64>,
    block_to: Option<u64>,
    topics0: Option<String>,
    topics1: Option<String>,
    topics2: Option<String>,
    topics3: Option<String>,
    addresses: Option<String>,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
    bytes: Option<bool>,
}

impl TryFrom<String> for ReadLogsQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct ReadContractQueryParams {
    chain_id: u64,
    function_signature: String,
    contract_addr: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
    bytes: Option<bool>,
}

impl TryFrom<String> for ReadContractQueryParams {
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

pub async fn get_xrc_data(req: HttpRequest) -> HttpResponse {
    let resp = _get_xrc_data(req, false).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_xrc_data_with_proof(req: HttpRequest) -> HttpResponse {
    let resp = _get_xrc_data(req, true).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _get_xrc_data(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = GetXRCDataQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let payer = resolve_payer(
        &req,
        "get_xrc_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    let rate = crate::methods::_get_xrc_data(params.id, with_signature, payer).await?;

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
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

    let payer = resolve_payer(
        &req,
        "get_asset_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    let rate = _get_asset_data(params.id, with_signature, payer).await?;

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
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

    let payer = resolve_payer(
        &req,
        "get_multiple_assets_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    let ids = params.ids.split(",").map(|s| s.to_string()).collect();

    let rate = _get_multiple_assets_data(ids, with_signature, None).await?;

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
    }
}

pub async fn gather_metrics() -> HttpResponse {
    let data = crate::utils::metrics::gather_metrics();

    response::ok(data)
}

pub async fn read_contract(req: HttpRequest) -> HttpResponse {
    let resp = _read_contract(req, false).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn read_contract_with_proof(req: HttpRequest) -> HttpResponse {
    let resp = _read_contract(req, true).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _read_contract(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = ReadContractQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let payer = resolve_payer(
        &req,
        "read_contract".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    let result = crate::methods::_read_contract(
        params.chain_id,
        params.function_signature,
        params.contract_addr,
        params.method,
        params.params,
        None,
        with_signature,
    )
    .await?;

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&result.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&result)?)
    }
}

pub async fn read_logs(req: HttpRequest) -> HttpResponse {
    let resp = _read_logs(req, false).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn read_logs_with_proof(req: HttpRequest) -> HttpResponse {
    let resp = _read_logs(req, true).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _read_logs(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = ReadLogsQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let payer = resolve_payer(
        &req,
        "read_logs".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    let result = crate::methods::_read_logs(
        params.chain_id,
        params.block_from,
        params.block_to,
        params
            .topics0
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        params
            .topics1
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        params
            .topics2
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        params
            .topics3
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        params
            .addresses
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        None,
        with_signature,
    )
    .await?;

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&result.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&result)?)
    }
}
