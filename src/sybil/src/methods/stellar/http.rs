use std::collections::HashMap;

use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpMethod,
};
use serde::Deserialize;

use crate::{log, methods::stellar::types::Request};

// TODO: decrease this value
const MAX_RESPONSE_BYTES_SIZE: u64 = 1_000_000;

pub async fn post<T: for<'a> Deserialize<'a>>(
    url: &str,
    method: &str,
    params: HashMap<String, serde_json::Value>,
) -> Result<T, anyhow::Error> {
    let request = Request {
        jsonrpc: "2.0",
        id: 1,
        method: method.to_string(),
        params,
    };

    log!("Request {:#?}", serde_json::to_string(&request).unwrap());

    let request = CanisterHttpRequestArgument {
        method: HttpMethod::POST,
        url: url.to_string(),
        body: Some(serde_json::to_vec(&request).unwrap()),
        headers: vec![HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        }],
        max_response_bytes: Some(MAX_RESPONSE_BYTES_SIZE),
        ..Default::default()
    };

    let response =
        match ic_cdk::api::management_canister::http_request::http_request(request, 1_000_000_000)
            .await
        {
            //4. DECODE AND RETURN THE RESPONSE
            Ok((response,)) => response,
            Err((_, m)) => {
                panic!("Error: {:?}", m);
            }
        };

    log!("Ress {:?}", response);
    log!(
        "Ress {:?}",
        String::from_utf8(response.body.clone()).unwrap()
    );

    let data: T = serde_json::from_slice(&response.body).unwrap();

    Ok(data)
}
