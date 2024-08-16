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
        feed_types::get_dxr_data::{DexType, GetDXRDataResult},
        http::{HttpRequest, HttpResponse},
    },
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetDXRDataQueryParams {
    pub chain_id: u64,
    pub pool_address: String,
    pub block_numbers: Option<String>,
    pub dex_type: DexType,
    pub reverse_pair: Option<bool>,
    pub api_key: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub bytes: Option<bool>,
    pub meta: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for GetDXRDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

pub async fn get_dxr_data(req: HttpRequest) -> HttpResponse {
    let resp = _get_dxr_data(req, false).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_dxr_data_with_proof(req: HttpRequest) -> HttpResponse {
    let resp = _get_dxr_data(req, true).await.map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _get_dxr_data(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = GetDXRDataQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let domain = get_domain(&req).unwrap();

    let (payer, is_free) = resolve_payer(
        Some(domain.clone()),
        "get_dxr_data".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    let block_numbers = params
        .block_numbers
        .map(|s| s.split(",").map(|s| s.parse().unwrap()).collect());

    let func_signature = stringify_func_call!(_get_dxr_data(
        params.chain_id,
        params.pool_address,
        block_numbers,
        params.dex_type,
        params.reverse_pair,
        with_signature
    ));

    let mut cache_builder = Cache::with(
        func_signature.clone(),
        crate::methods::feed_methods::get_dxr_data::_get_dxr_data(
            params.chain_id.clone(),
            params.pool_address.clone(),
            block_numbers.clone(),
            params.dex_type.clone(),
            params.reverse_pair.clone(),
            with_signature,
            if is_free { None } else { payer.clone() },
        ),
    );

    cache_builder.with_on_found(|_| {
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(api_key, "get_dxr_data".to_string(), Some(domain))
                .unwrap();
        } else {
            if Allowances::get_allowed_user(&domain).is_some() {
                Allowances::decrease_request_count(domain, "get_dxr_data".to_string()).unwrap();
            }
        }
    });

    cache_builder.with_on_save(async move {
        ic_cdk::spawn(async move {
            let entry =
                Cache::get_cache::<GetDXRDataResult>(func_signature.clone(), params.cache_ttl);

            if let Some(mut data) = entry {
                data.meta.as_mut().map(|meta| meta.fee = 0.into());
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

    let mut result = cache_builder.evaluate().await?;

    if is_free {
        result.meta.as_mut().map(|meta| meta.fee = 0.into());
        if with_signature {
            result.sign().await?;
        }
    }

    if !params.meta.unwrap_or(false) {
        result.meta = None;
    }

    if params.bytes.unwrap_or(false) {
        result.bytes = Some(format!("0x{}", hex::encode(&result.encode())));
    }

    Ok(serde_json::to_vec(&result)?)
}
