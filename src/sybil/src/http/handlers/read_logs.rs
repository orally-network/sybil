use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    http::{response, utils::resolve_payer, HTTP_SERVICE},
    types::http::{HttpRequest, HttpResponse},
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct ReadLogsQueryParams {
    pub chain_id: u64,
    pub block_from: Option<u64>,
    pub block_to: Option<u64>,
    pub topics0: Option<String>,
    pub topics1: Option<String>,
    pub topics2: Option<String>,
    pub topics3: Option<String>,
    pub addresses: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for ReadLogsQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
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

    let mut result = crate::methods::feed_methods::read_logs::_read_logs(
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
        if is_free { None } else { payer.clone() },
        with_signature,
        params.cache_ttl,
    )
    .await?;

    if is_free || payer.is_some() {
        result.meta.fee = 0.into();
    }

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&result.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&result)?)
    }
}
