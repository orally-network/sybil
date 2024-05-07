use candid::{CandidType, Func};
use serde::Deserialize;
use serde_bytes::ByteBuf;

pub type HeaderField = (String, String);

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Token {}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub enum StreamingStrategy {
    Callback { callback: Func, token: Token },
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct StreamingCallbackHttpResponse {
    pub body: ByteBuf,
    pub token: Option<Token>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: ByteBuf,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub upgrade: Option<bool>,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}

#[derive(Clone, Debug, Default)]
pub struct APIRequest {
    pub method: String,
    pub count: u128,
    pub domain: String,
    pub last_request: u64,
}
