mod http;
mod jobs;
mod methods;
mod migrations;
mod types;
mod utils;

use std::cell::RefCell;

use http::HttpService;
use ic_cdk::{
    api::management_canister::http_request::{HttpResponse, TransformArgs},
    init, query,
};
use types::{
    cache::RateCache,
    config::Cfg,
    state::{self, State},
};

use crate::types::cache::{HttpCache, SignaturesCache};

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();
    pub static CACHE: RefCell<RateCache> = RefCell::default();
    pub static HTTP_CACHE: RefCell<HttpCache> = RefCell::default();
    pub static SIGNATURES_CACHE: RefCell<SignaturesCache> = RefCell::default();
}

#[query]
fn transform(response: TransformArgs) -> HttpResponse {
    response.response
}

#[init]
pub fn init(cfg: Cfg) {
    state::init(&cfg);

    HttpService::init();
}
