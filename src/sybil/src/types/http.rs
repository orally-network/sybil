use std::borrow::Cow;

use serde_bytes::{ByteBuf, Bytes};

use ic_cdk::export::candid::{CandidType, Deserialize, Func};

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
    pub body: Cow<'static, Bytes>,
    pub streaming_strategy: Option<StreamingStrategy>,
}