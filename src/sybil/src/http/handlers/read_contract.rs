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
pub struct ReadContractQueryParams {
    pub chain_id: u64,
    pub function_signature: String,
    pub contract_addr: String,
    pub method: String,
    pub params: String,
    pub block_number: Option<u64>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for ReadContractQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
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

    let domain = get_domain(&req);

    let (payer, is_free) = resolve_payer(
        domain.clone(),
        "read_contract".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    let func_signature = stringify_func_call!(_read_contract(
        params.chain_id,
        params.function_signature,
        params.contract_addr,
        params.method,
        params.params,
        params.block_number,
        with_signature
    ));

    let mut cache_builder = Cache::with(
        func_signature,
        crate::methods::feed_methods::read_contract::_read_contract(
            params.chain_id,
            params.function_signature,
            params.contract_addr,
            params.method,
            params.params,
            params.block_number,
            if is_free { None } else { payer.clone() },
            with_signature,
        ),
    );

    cache_builder.with_on_found(|r| {
        r.meta.fee = 0.into();
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(api_key, "read_contract".to_string(), domain).unwrap();
        } else {
            Allowances::decrease_request_count(domain.unwrap(), "read_contract".to_string())
                .unwrap();
        }
    });

    if let Some(cache_ttl) = params.cache_ttl {
        cache_builder.with_cache_ttl(cache_ttl);
    }

    let mut result = cache_builder.evaluate().await?;

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
