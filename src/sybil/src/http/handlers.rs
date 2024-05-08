use anyhow::{Context, Result};
use validator::Validate;

use super::{
    response,
    types::{
        GetAssetDataQueryParams, GetMultipleAssetsDataQueryParams, GetXRCDataQueryParams,
        ReadContractQueryParams, ReadLogsQueryParams,
    },
    utils::resolve_payer,
    HttpRequest, HttpResponse, HTTP_SERVICE,
};

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

    let (payer, is_free) = resolve_payer(
        &req,
        "get_xrc_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    if payer.is_none() && !is_free {
        return Err(anyhow::anyhow!("No payer provided"));
    }

    let rate =
        crate::methods::feed_methods::get_xrc_data::_get_xrc_data(params.id, with_signature, None)
            .await?;

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

    let (payer, is_free) = resolve_payer(
        &req,
        "get_asset_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    if payer.is_none() && !is_free {
        return Err(anyhow::anyhow!("No payer provided"));
    }

    let rate = crate::methods::feed_methods::get_asset_data::_get_asset_data(
        params.id,
        with_signature,
        None,
    )
    .await?;

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

    let (payer, is_free) = resolve_payer(
        &req,
        "get_multiple_assets_data".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    if payer.is_none() && !is_free {
        return Err(anyhow::anyhow!("No payer provided"));
    }

    let ids = params.ids.split(",").map(|s| s.to_string()).collect();

    let rate = crate::methods::feed_methods::get_multiple_asset_data::_get_multiple_assets_data(
        ids,
        with_signature,
        None,
    )
    .await?;

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

    let (payer, is_free) = resolve_payer(
        &req,
        "read_contract".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    if payer.is_none() && !is_free {
        return Err(anyhow::anyhow!("No payer provided"));
    }

    let result = crate::methods::feed_methods::read_contract::_read_contract(
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

    let (payer, is_free) = resolve_payer(
        &req,
        "read_logs".to_string(),
        params.msg,
        params.sig,
        params.api_key,
    )
    .await?;

    if payer.is_none() && !is_free {
        return Err(anyhow::anyhow!("No payer provided"));
    }

    let result = crate::methods::feed_methods::read_logs::_read_logs(
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
