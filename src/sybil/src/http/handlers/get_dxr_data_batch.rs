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
        feed_types::get_dxr_data::{Aggregation, DexType, GetDXRDataBatchResult},
        http::{HttpRequest, HttpResponse},
    },
};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetDXRDataBatchQueryParams {
    pub chain_id: u64,
    pub pool_addresses: Vec<String>,
    pub aggregation: Option<Aggregation>,
    pub dex_type: DexType,
    pub reverse_pair: Option<bool>,
    pub api_key: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub bytes: Option<bool>,
    pub meta: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for GetDXRDataBatchQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

pub async fn get_dxr_data_batch(req: HttpRequest) -> HttpResponse {
    let resp = _get_dxr_data_batch(req, false)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

pub async fn get_dxr_data_batch_with_proof(req: HttpRequest) -> HttpResponse {
    let resp = _get_dxr_data_batch(req, true)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _get_dxr_data_batch(req: HttpRequest, with_signature: bool) -> Result<Vec<u8>> {
    let service = HTTP_SERVICE.get().expect("State not initialized");

    let query = service
        .update_router
        .inner
        .at(&req.url)
        .context("No route found")?
        .params;

    let params = GetDXRDataBatchQueryParams::try_from(query.to_string())?;
    params.validate()?;

    let with_meta = params.meta.unwrap_or(false);

    let domain = get_domain(&req).ok_or(anyhow::anyhow!("Domain not found"))?;

    let (payer, is_free) = resolve_payer(
        Some(domain.clone()),
        "get_dxr_data_batch".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    if payer.is_none() {
        return Err(anyhow::anyhow!("Payer not found"));
    }

    let func_signature = stringify_func_call!(_get_dxr_data_batch(
        params.chain_id,
        params.pool_addresses,
        params.aggregation,
        params.dex_type,
        params.reverse_pair,
        with_signature,
        with_meta
    ));

    let mut cache_builder = Cache::with(
        func_signature.clone(),
        crate::methods::feed_methods::get_dxr_data::_get_dxr_data_batch(
            params.chain_id.clone(),
            &params.pool_addresses,
            params.aggregation,
            params.dex_type.clone(),
            params.reverse_pair.clone(),
            with_signature,
            with_meta,
            if is_free { None } else { payer.clone() },
        ),
    );

    cache_builder.with_on_found(|_| {
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(
                api_key,
                "get_dxr_data_batch".to_string(),
                Some(domain),
            )
            .unwrap();
        } else {
            if Allowances::get_allowed_user(&domain).is_some() {
                Allowances::decrease_request_count(domain, "get_dxr_data_batch".to_string())
                    .unwrap();
            }
        }
    });

    cache_builder.with_on_save(async move {
        ic_cdk::spawn(async move {
            let entry =
                Cache::get_cache::<GetDXRDataBatchResult>(func_signature.clone(), params.cache_ttl);

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

    if params.bytes.unwrap_or(false) {
        result.bytes = Some(format!("0x{}", hex::encode(&result.encode())));
    }

    Ok(serde_json::to_vec(&result)?)
}
