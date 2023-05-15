pub mod encoding;
pub mod update_rate;

use std::str::FromStr;

use ic_cdk::{
    api::management_canister::http_request::{
        http_request, CanisterHttpRequestArgument, HttpMethod,
    },
    export::{candid::Nat, Principal},
};
use ic_web3::types::H160;

use anyhow::{anyhow, Context, Result};
use url::Url;

use crate::{types::rate_data::CustomPairData, CACHE, STATE};

pub fn nat_to_u64(nat: Nat) -> u64 {
    *nat.0
        .to_u64_digits()
        .last()
        .expect("should be at least one digit")
}

pub fn is_pair_exist(pair_id: &str) -> bool {
    STATE.with(|state| {
        let pairs = &state.borrow().pairs;
        let custom_pairs = &state.borrow().custom_pairs;

        pairs.iter().any(|pair| pair.id == pair_id)
            || custom_pairs.iter().any(|pair| pair.id == pair_id)
    })
}

pub async fn rec_eth_addr(msg: &str, sig: &str) -> Result<H160> {
    let siwe_canister = STATE.with(|state| state.borrow().siwe_signer_canister.clone());

    let siwe_canister = Principal::from_text(siwe_canister).expect("canister should be valid");

    let msg = msg.to_string();
    let sig = sig.to_string();

    let (signer,): (String,) = ic_cdk::call(siwe_canister, "get_signer", (msg, sig))
        .await
        .map_err(|(code, msg)| anyhow!("{:?}: {}", code, msg))?;

    H160::from_str(&signer).context("failed to parse signer address")
}

pub async fn get_rate_with_cache(url: &Url) -> Result<(CustomPairData, u64)> {
    let response = CACHE.with(|cache| cache.borrow_mut().get_entry(url.as_ref()));

    if let Some(response) = response {
        return Ok((serde_json::from_slice(&response)?, response.len() as u64));
    }

    let request_args = CanisterHttpRequestArgument {
        url: url.to_string(),
        method: HttpMethod::GET,
        max_response_bytes: None,
        headers: vec![],
        body: None,
        transform: None,
    };

    let (response,) = http_request(request_args)
        .await
        .map_err(|(code, msg)| anyhow!("Failed to make a request: {}, {:?}", msg, code))?;

    let rate: CustomPairData = serde_json::from_slice(&response.body)?;

    CACHE.with(|cache| {
        cache
            .borrow_mut()
            .add_entry(url.to_string(), response.body.clone())
    });

    Ok((rate, response.body.len() as u64))
}
