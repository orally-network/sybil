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
        feed_types::get_multiple_asset_data::GetMultipleAssetDataResult,
        http::{HttpRequest, HttpResponse},
    },
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetMultipleAssetsDataQueryParams {
    pub ids: String,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for GetMultipleAssetsDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
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

    let domain = get_domain(&req).unwrap();

    let (payer, is_free) = resolve_payer(
        Some(domain.clone()),
        "get_multiple_assets_data".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    let ids: Vec<_> = params.ids.split(",").map(|s| s.to_string()).collect();

    let func_signature =
        stringify_func_call!(_get_multiple_assets_data_result(ids, with_signature));
    let mut cache_builder = Cache::with(
        func_signature.clone(),
        crate::methods::feed_methods::get_multiple_asset_data::_get_multiple_assets_data_result(
            ids.clone(),
            with_signature,
            if is_free { None } else { payer.clone() },
        ),
    );

    cache_builder.with_on_found(|_| {
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(
                api_key,
                "get_multiple_assets_data".to_string(),
                Some(domain),
            )
            .unwrap();
        } else {
            if Allowances::get_allowed_user(&domain).is_some() {
                Allowances::decrease_request_count(domain, "get_multiple_assets_data".to_string())
                    .unwrap();
            }
        }
    });

    cache_builder.with_on_save(async move {
        ic_cdk::spawn(async move {
            let entry = Cache::get_cache::<GetMultipleAssetDataResult>(
                func_signature.clone(),
                params.cache_ttl,
            );

            if let Some(mut data) = entry {
                data.meta.fee = 0.into();
                if with_signature {
                    data.sign().await.unwrap();
                }

                Cache::save_cache(func_signature, data, params.cache_ttl);
            }
        })
    });

    if let Some(cache_ttl) = params.cache_ttl {
        cache_builder.with_cache_ttl(cache_ttl);
    }

    let mut rate = cache_builder.evaluate().await?;

    if is_free {
        rate.meta.fee = 0.into();
        if with_signature {
            rate.sign().await?;
        }
    }

    if let Some(_bytes @ true) = params.bytes {
        let data = format!("0x{}", hex::encode(&rate.encode()));
        Ok(serde_json::to_vec(&data)?)
    } else {
        Ok(serde_json::to_vec(&rate)?)
    }
}
