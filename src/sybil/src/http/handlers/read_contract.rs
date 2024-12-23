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
        balances::Balances,
        cache::Cache,
        feed_types::read_contract::ReadContractResult,
        http::{HttpRequest, HttpResponse},
        state::get_cfg,
    },
    utils::canister,
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
    pub meta: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl TryFrom<String> for ReadContractQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        let decoded = urlencoding::decode(&query)?;
        serde_qs::from_str(&decoded)
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

    let with_meta = params.meta.unwrap_or(false);

    let domain = get_domain(&req).unwrap();

    let (payer, is_free) = resolve_payer(
        Some(domain.clone()),
        "read_contract".to_string(),
        params.msg,
        params.sig,
        params.api_key.clone(),
    )
    .await?;

    if payer.is_none() {
        return Err(anyhow::anyhow!("Payer not found"));
    }

    let payer = if is_free { None } else { payer };

    let func_signature = stringify_func_call!(_read_contract(
        params.chain_id,
        params.function_signature,
        params.contract_addr,
        params.method,
        params.params,
        params.block_number,
        with_meta
    ));

    let mut cache_builder = Cache::with(
        func_signature.clone(),
        crate::methods::feed_methods::read_contract::_read_contract(
            params.chain_id,
            params.function_signature,
            params.contract_addr,
            params.method,
            params.params,
            params.block_number,
            payer.clone(),
            with_signature,
            with_meta,
        ),
    );

    cache_builder.with_on_found(|_| {
        if let Some(api_key) = params.api_key {
            APIKeys::decrease_request_count(api_key, "read_contract".to_string(), Some(domain))
                .unwrap();
        } else {
            if Allowances::get_allowed_user(&domain).is_some() {
                Allowances::decrease_request_count(domain, "read_contract".to_string()).unwrap();
            }
        }
    });

    cache_builder.with_on_save(async move {
        ic_cdk::spawn(async move {
            let entry =
                Cache::get_cache::<ReadContractResult>(func_signature.clone(), params.cache_ttl);

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

    // Sign the result if needed
    if result.signature.is_none() && with_signature {
        result.sign().await?;
        if let Some(payer) = &payer {
            let signature_fee = get_cfg().balances_cfg.signature_fee;
            Balances::reduce_amount(payer, &signature_fee)?;
            Balances::add_amount(&canister::eth_address().await?, &signature_fee)?;
        }
    }

    // Remove signature if not needed
    if result.signature.is_some() && !with_signature {
        result.signature = None;
    }

    if params.bytes.unwrap_or(false) {
        result.bytes = Some(format!("0x{}", hex::encode(result.encode())));
    }

    Ok(serde_json::to_vec(&result)?)
}
