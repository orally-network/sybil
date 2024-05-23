use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    http::{response, utils::resolve_payer, HTTP_SERVICE},
    types::http::{HttpRequest, HttpResponse},
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetXRCDataQueryParams {
    pub id: String,
    pub api_key: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub bytes: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for GetXRCDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
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

    let mut rate = crate::methods::feed_methods::get_xrc_data::_get_xrc_data(
        params.id.clone(),
        with_signature,
        if is_free { None } else { payer.clone() },
        params.cache_ttl,
    )
    .await?;

    if is_free || payer.is_some() {
        rate.meta.fee = 0.into();
    }

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
    }
}
