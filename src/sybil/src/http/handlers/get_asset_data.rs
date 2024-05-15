use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    http::{response, utils::resolve_payer, HTTP_SERVICE},
    types::{
        feed_types::get_asset_data::{GetAssetDataMetadata, GetAssetDataResult},
        http::{HttpRequest, HttpResponse},
    },
    utils::time::in_seconds,
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetAssetDataQueryParams {
    pub id: String,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for GetAssetDataQueryParams {
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
        params.id.clone(),
        false,
        None,
        params.cache_ttl,
    )
    .await?;

    let mut rate = GetAssetDataResult {
        data: rate.data,
        meta: GetAssetDataMetadata {
            id: params.id.clone(),
            timestamp: in_seconds(),
        },
        signature: None,
    };

    if with_signature {
        rate.sign().await?;
    }

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
    }
}
