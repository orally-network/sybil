use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{response, HttpRequest, HttpResponse, HTTP_SERVICE};
use crate::{
    log,
    methods::{_get_asset_data, _get_multiple_assets_data},
    types::allowances::Allowances,
};

use crate::utils::siwe;

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAssetDataQueryParams {
    id: String,
    msg: Option<String>,
    sig: Option<String>,
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
struct ReadContractQueryParams {
    chain_id: u64,
    function_signature: String,
    contract_addr: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
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

    let caller = if let (Some(msg), Some(sig)) = (params.msg, params.sig) {
        siwe::recover(&msg, &sig).await?
    } else {
        ic_cdk::caller().to_string()
    };

    let payer = check_for_potential_grantee(&req.headers)?.unwrap_or(caller);

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

    let caller = if let (Some(msg), Some(sig)) = (params.msg, params.sig) {
        siwe::recover(&msg, &sig).await?
    } else {
        ic_cdk::caller().to_string()
    };

    let payer = check_for_potential_grantee(&req.headers)?.unwrap_or(caller);

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

    let caller = if let (Some(msg), Some(sig)) = (params.msg, params.sig) {
        siwe::recover(&msg, &sig).await?
    } else {
        ic_cdk::caller().to_string()
    };

    let payer = check_for_potential_grantee(&req.headers)?.unwrap_or(caller);

    let ids = params.ids.split(",").map(|s| s.to_string()).collect();

    let rate = _get_multiple_assets_data(ids, with_signature, payer).await?;

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

fn check_for_potential_grantee(headers: &[(String, String)]) -> Result<Option<String>> {
    let grantee = headers
        .iter()
        .find(|(k, _)| k == "referer" || k == "origin")
        .map(|(_, v)| v);

    if let Some(grantee) = grantee {
        Ok(Allowances::get_allowed_user(grantee)?)
    } else {
        Ok(None)
    }
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

    let caller = if let (Some(msg), Some(sig)) = (params.msg, params.sig) {
        siwe::recover(&msg, &sig).await?
    } else {
        ic_cdk::caller().to_string()
    };

    let payer = check_for_potential_grantee(&req.headers)?.unwrap_or(caller);

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
