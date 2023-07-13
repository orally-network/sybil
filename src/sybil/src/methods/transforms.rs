use ic_cdk::{
    api::management_canister::http_request::{HttpResponse, TransformArgs},
    query,
};
use ic_web3_rs::transforms::{processors, transform::TransformProcessor};

use crate::utils;

#[query]
fn transform_tx_with_logs(args: TransformArgs) -> HttpResponse {
    utils::processors::raw_tx_execution_transform_processor().transform(args)
}

#[query]
fn transform_tx(args: TransformArgs) -> HttpResponse {
    processors::send_transaction_processor().transform(args)
}
