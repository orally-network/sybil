mod methods;
mod types;
mod utils;

use std::cell::RefCell;

use ic_cdk::{
    api::management_canister::http_request::{HttpResponse, TransformArgs},
    query,
};

use crate::types::{cache::Cache, state::State};

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();
    pub static CACHE: RefCell<Cache> = RefCell::default();
}

#[query]
fn transform(response: TransformArgs) -> HttpResponse {
    response.response
}
