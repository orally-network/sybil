use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    http::{
        response,
        utils::{get_domain, resolve_payer},
        HTTP_SERVICE,
    },
    stringify_func_call,
    types::{
        allowances::Allowances,
        api_keys::APIKeys,
        cache::Cache,
        http::{HttpRequest, HttpResponse},
    },
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

    let domain = get_domain(&req);

    let (payer, is_free) = resolve_payer(
        domain.clone(),
        "get_asset_data".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    let func_signature = stringify_func_call!(_get_asset_data_result(params.id, with_signature));
    let mut cache_builder = Cache::with(
        func_signature,
        crate::methods::feed_methods::get_asset_data::_get_asset_data_result(
            params.id.clone(),
            with_signature,
            if is_free { None } else { payer.clone() },
        ),
    );

    cache_builder.with_on_found(|r| {
        r.meta.fee = 0.into();
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(api_key, "get_asset_data".to_string(), domain).unwrap();
        } else {
            Allowances::decrease_request_count(domain.unwrap(), "get_asset_data".to_string())
                .unwrap();
        }
    });

    if let Some(cache_ttl) = params.cache_ttl {
        cache_builder.with_cache_ttl(cache_ttl);
    }

    let mut rate = cache_builder.evaluate().await?;

    if is_free || payer.is_some() {
        rate.meta.fee = 0.into();
    }

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
