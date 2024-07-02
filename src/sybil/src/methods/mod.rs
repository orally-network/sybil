pub mod allowances;
pub mod api_keys;
pub mod balances;
pub mod chains_rpc;
pub mod controllers;
pub mod custom_feeds;
pub mod default_feeds;
pub mod feed_methods;
pub mod signatures;
pub mod transforms;
pub mod whitelist;

pub mod stellar;

use std::{collections::HashMap, fmt::format, str::FromStr};

use base64::Engine;
use candid::{de, CandidType, Principal};
use ethers_core::k256::{
    pkcs8::der::pem::Base64Decoder,
    sha2::{Digest, Sha256},
};
use ic_cdk::{
    api::management_canister::http_request::{
        CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext, TransformFunc,
    },
    query, update,
};

use ic_web3_rs::ic::pubkey_to_address;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use stellar_strkey::ed25519::PublicKey;
use stellar_xdr::next::{
    AccountEntry, AccountId, Asset, DecoratedSignature, LedgerEntryData, LedgerKey,
    LedgerKeyAccount, Limits, Memo, MuxedAccount, Operation, OperationBody, PaymentOp,
    Preconditions, ReadXdr, Signature, SignatureHint, Transaction, TransactionEnvelope,
    TransactionExt, TransactionResult, TransactionV1Envelope, Uint256, VecM, WriteXdr,
};
use thiserror::Error;

use ic_utils::{
    api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
    get_information, update_information,
};

use crate::{
    clone_with_state, log,
    types::{
        feeds::{Feed, FeedError, FeedStorage, GetFeedsFilter},
        pagination::{Pagination, PaginationResult},
    },
    utils::{canister, processors::transform_ctx, siwe},
};

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Feed error: {0}")]
    FeedError(#[from] FeedError),
    #[error("Siwe error: {0}")]
    SiweError(#[from] siwe::SiweError),
}

#[query]
fn is_feed_exists(id: String) -> bool {
    FeedStorage::contains(&id)
}

#[query]
async fn get_feed(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<Option<Feed>, String> {
    _get_feed(id, msg, sig)
        .await
        .map_err(|e| format!("failed to get feed: {}", e))
}

async fn _get_feed(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<Option<Feed>, AssetsError> {
    let caller = if let (Some(msg), Some(sig)) = (msg, sig) {
        Some(siwe::recover(&msg, &sig).await?)
    } else {
        None
    };

    if let Some(mut feed) = FeedStorage::get(&id) {
        feed.censor_if_needed(&caller);
        Ok(Some(feed))
    } else {
        Ok(None)
    }
}

#[query]
async fn get_feeds(
    filter: Option<GetFeedsFilter>,
    pagination: Option<Pagination>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<PaginationResult<Feed>, String> {
    _get_feeds(filter, pagination, msg, sig)
        .await
        .map_err(|e| format!("failed to get feeds: {}", e))
}

async fn _get_feeds(
    filter: Option<GetFeedsFilter>,
    pagination: Option<Pagination>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<PaginationResult<Feed>, AssetsError> {
    let caller = if let (Some(msg), Some(sig)) = (msg, sig) {
        Some(siwe::recover(&msg, &sig).await?)
    } else {
        None
    };

    let mut feeds = FeedStorage::get_all(filter);

    feeds
        .iter_mut()
        .for_each(|feed| feed.censor_if_needed(&caller));

    match pagination {
        Some(pagination) => {
            feeds.sort_by(|l, r| l.id.cmp(&r.id));
            Ok(pagination.paginate(feeds))
        }
        None => Ok(feeds.into()),
    }
}

#[query(name = "getCanistergeekInformation")]
pub async fn get_canistergeek_information(
    request: GetInformationRequest,
) -> GetInformationResponse<'static> {
    get_information(request)
}

#[update(name = "updateCanistergeekInformation")]
pub async fn update_canistergeek_information(request: UpdateInformationRequest) {
    update_information(request);
}

#[update]
pub async fn eth_address() -> Result<String, String> {
    canister::eth_address()
        .await
        .map_err(|e| format!("failed to get eth address: {}", e))
}
