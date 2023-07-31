use std::borrow::Cow;

use serde_json::json;

use crate::types::http::HttpResponse;

#[inline(always)]
pub fn ok(body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status_code: 200,
        upgrade: Some(false),
        headers: vec![("content-type".into(), "application/json".into())],
        body: Cow::Owned(serde_bytes::ByteBuf::from(body)),
        streaming_strategy: None,
    }
}

#[inline(always)]
pub fn bad_request(msg: String) -> HttpResponse {
    let error = json!({
        "error": msg,
    });

    HttpResponse {
        status_code: 400,
        upgrade: Some(false),
        headers: vec![("content-type".into(), "application/json".into())],
        body: Cow::Owned(serde_bytes::ByteBuf::from(error.to_string().as_bytes())),
        streaming_strategy: None,
    }
}

#[inline(always)]
pub fn page_not_found(upgrade: bool) -> HttpResponse {
    HttpResponse {
        status_code: 404,
        upgrade: Some(upgrade),
        headers: vec![],
        body: Cow::Owned(serde_bytes::ByteBuf::from("Page not found".as_bytes())),
        streaming_strategy: None,
    }
}
