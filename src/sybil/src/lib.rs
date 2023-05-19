mod methods;
mod migrations;
mod types;
mod utils;

use std::{borrow::Cow, cell::RefCell};

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use ic_cdk::{
    api::management_canister::http_request::{HttpResponse, TransformArgs},
    query, update,
};

use crate::{
    methods::get_asset_data_with_proof,
    types::{
        cache::Cache,
        http::{HttpRequest as CandidHttpRequest, HttpResponse as CandidHttpResponse},
        state::State,
    },
    utils::{is_pair_exist, is_valid_pair_id},
};

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();
    pub static CACHE: RefCell<Cache> = RefCell::default();
}

#[query]
fn transform(response: TransformArgs) -> HttpResponse {
    response.response
}

#[query]
pub fn http_request(req: CandidHttpRequest) -> CandidHttpResponse {
    let upgrade = matches!(&req.url, url if url.starts_with("/get_asset_data_with_proof?pair_id="));

    get_page_not_found(upgrade)
}

#[update]
pub async fn http_request_update(req: CandidHttpRequest) -> CandidHttpResponse {
    match &req.url {
        url if url.starts_with("/get_asset_data_with_proof?pair_id=") => {
            handle_get_asset_data_with_proof_request(req).await
        }
        _ => get_page_not_found(false),
    }
}

async fn handle_get_asset_data_with_proof_request(req: CandidHttpRequest) -> CandidHttpResponse {
    let resp = _handle_get_asset_data_with_proof_request(req)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => get_ok(data),
        Err(err) => get_bad_request(err),
    }
}

async fn _handle_get_asset_data_with_proof_request(req: CandidHttpRequest) -> Result<Vec<u8>> {
    let pair_id = req
        .url
        .strip_prefix("/get_asset_data_with_proof?pair_id=")
        .context("invalid query")?;

    if !is_valid_pair_id(pair_id) {
        return Err(anyhow!("invalid pair_id"));
    }

    let (is_exist, _) = is_pair_exist(pair_id);
    if !is_exist {
        return Err(anyhow!("pair_id does not exist"));
    }

    let asset = get_asset_data_with_proof(pair_id.into())
        .await
        .map_err(|e| anyhow!(e))?;

    Ok(serde_json::to_vec(&asset)?)
}

fn get_ok(body: Vec<u8>) -> CandidHttpResponse {
    CandidHttpResponse {
        status_code: 200,
        upgrade: Some(false),
        headers: vec![("content-type".into(), "application/json".into())],
        body: Cow::Owned(serde_bytes::ByteBuf::from(body)),
        streaming_strategy: None,
    }
}

fn get_bad_request(msg: String) -> CandidHttpResponse {
    let error = json!({
        "error": msg,
    });

    CandidHttpResponse {
        status_code: 400,
        upgrade: Some(false),
        headers: vec![("content-type".into(), "application/json".into())],
        body: Cow::Owned(serde_bytes::ByteBuf::from(error.to_string().as_bytes())),
        streaming_strategy: None,
    }
}

fn get_page_not_found(upgrade: bool) -> CandidHttpResponse {
    CandidHttpResponse {
        status_code: 404,
        upgrade: Some(upgrade),
        headers: vec![],
        body: Cow::Owned(serde_bytes::ByteBuf::from("Page not found".as_bytes())),
        streaming_strategy: None,
    }
}
