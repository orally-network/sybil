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
        feed_types::read_logs::ReadLogsResult,
        http::{HttpRequest, HttpResponse},
    },
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

    let domain = get_domain(&req).unwrap();

    let (payer, is_free) = resolve_payer(
        Some(domain.clone()),
        "read_logs".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    let topics0 = params
        .topics0
        .map(|s| s.split(",").map(|s| s.to_string()).collect());

    let topics1 = params
        .topics1
        .map(|s| s.split(",").map(|s| s.to_string()).collect());

    let topics2 = params
        .topics2
        .map(|s| s.split(",").map(|s| s.to_string()).collect());

    let topics3 = params
        .topics3
        .map(|s| s.split(",").map(|s| s.to_string()).collect());

    let addresses = params
        .addresses
        .map(|s| s.split(",").map(|s| s.to_string()).collect());

    let func_signature = stringify_func_call!(_read_logs(
        params.chain_id,
        params.block_from,
        params.block_to,
        topics0,
        topics1,
        topics2,
        topics3,
        addresses,
        with_signature
    ));

    let mut cache_builder = Cache::with(
        func_signature.clone(),
        crate::methods::feed_methods::read_logs::_read_logs(
            params.chain_id,
            params.block_from,
            params.block_to,
            topics0,
            topics1,
            topics2,
            topics3,
            addresses,
            if is_free { None } else { payer.clone() },
            with_signature,
        ),
    );

    cache_builder.with_on_found(|_| {
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(api_key, "read_logs".to_string(), Some(domain))
                .unwrap();
        } else {
            if Allowances::get_allowed_user(&domain).is_some() {
                Allowances::decrease_request_count(domain, "read_logs".to_string()).unwrap();
            }
        }
    });

    cache_builder.with_on_save(async move {
        ic_cdk::spawn(async move {
            let entry =
                Cache::get_cache::<ReadLogsResult>(func_signature.clone(), params.cache_ttl);

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

    let mut result = cache_builder.evaluate().await?;

    if is_free {
        result.meta.fee = 0.into();
        if with_signature {
            result.sign().await?;
        }
    }

    if let Some(_bytes @ true) = params.bytes {
        result.bytes = Some(format!("0x{}", hex::encode(&result.encode())));
    }

    Ok(serde_json::to_vec(&result)?)
}
