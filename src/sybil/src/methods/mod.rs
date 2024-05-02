pub mod allowances;
pub mod api_keys;
pub mod balances;
pub mod chains_rpc;
pub mod controllers;
pub mod custom_feeds;
pub mod default_feeds;
pub mod signatures;
pub mod transforms;
pub mod whitelist;

use std::str::FromStr;

use futures::future::join_all;
use ic_cdk::{query, update};

use ic_web3_rs::{contract::Contract, types::H256};
use thiserror::Error;

use ic_utils::{
    api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
    get_information, update_information,
};

use crate::{
    clone_with_state, metrics,
    types::{
        balances::{BalanceError, Balances},
        chains_rpc::ChainsRPC,
        feeds::{Feed, FeedError, FeedStorage, GetFeedsFilter, DEFAULT_UPDATE_FREQ},
        pagination::{Pagination, PaginationResult},
        rate_data::{AssetDataResult, MultipleAssetsDataResult},
        read_contract::{ReadContractMetadata, ReadContractResult, SolidityToken},
        read_logs::{ReadLogsData, ReadLogsMetadata, ReadLogsResult},
        state,
    },
    utils::{address, canister, encoding::parse_tokens, siwe, time::in_seconds, web3},
};

use self::custom_feeds::CustomFeedError;

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

#[update]
pub async fn get_asset_data_with_proof(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_asset_data(id, true, Some(payer))
        .await
        .map_err(|e| format!("failed to get asset data with proof: {}", e))
}

#[update]
pub async fn read_contract_with_proof(
    chain_id: u64,
    function_signature: String,
    contract_address: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadContractResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_contract(
        chain_id,
        function_signature,
        contract_address,
        method,
        params,
        None,
        true,
    )
    .await
    .map_err(|e| format!("Failed to read contract: {e}"))
}

#[update]
pub async fn read_contract(
    chain_id: u64,
    function_signature: String,
    contract_address: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadContractResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_contract(
        chain_id,
        function_signature,
        contract_address,
        method,
        params,
        None,
        false,
    )
    .await
    .map_err(|e| format!("Failed to read contract: {e}"))
}

#[inline]
pub async fn _read_contract(
    chain_id: u64,
    function_signature: String,
    contract_addr: String,
    method: String,
    params: String,
    payer: Option<String>,
    with_signature: bool,
) -> Result<ReadContractResult, CustomFeedError> {
    let base_fee = state::get_cfg().balances_cfg.base_fee;

    if let Some(ref payer) = payer {
        if !Balances::is_sufficient(payer, &base_fee)? {
            return Err(BalanceError::InsufficientBalance)?;
        };
    }

    let chain_rpc = ChainsRPC::get_first_chain_rpc(chain_id)?;

    let w3 = web3::instance(chain_rpc, clone_with_state!(evm_rpc_canister));

    let contract_address = address::to_h160(&contract_addr)?;

    let ethabi_contract = ethers_core::abi::parse_abi_str(&function_signature)
        .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?;

    let contract = Contract::new(w3.eth(), contract_address, ethabi_contract);

    let function = contract
        .abi()
        .function(&method)
        .map_err(|_| CustomFeedError::AbiDoesntContainMethod(method.clone()))?;

    let inputs = function
        .inputs
        .iter()
        .map(|p| p.kind.clone())
        .collect::<Vec<_>>();

    let tokens = parse_tokens(&inputs, params[1..params.len() - 1].to_string())?;

    let from = canister::eth_address().await?.to_string();

    let call_result = w3
        .get_call_result(
            &contract,
            &method,
            &tokens,
            address::to_h160(&from)?,
            Some(contract_address),
            None, // tx_hash.block_number,
        )
        .await?;

    let mut result = ReadContractResult {
        data: call_result.into_iter().map(SolidityToken::from).collect(),
        meta: ReadContractMetadata {
            chain_id,
            contract_address: contract_addr,
            method,
            params,
            timestamp: in_seconds(),
        },
        signature: None,
    };

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &base_fee)?;
        Balances::add_amount(&canister::eth_address().await?, &base_fee)?;
    }

    Ok(result)
}

#[update]
pub async fn read_logs(
    chain_id: u64,
    block_from: Option<u64>,
    block_to: Option<u64>,
    topics0: Option<Vec<String>>,
    topics1: Option<Vec<String>>,
    topics2: Option<Vec<String>>,
    topics3: Option<Vec<String>>,
    addresses: Option<Vec<String>>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadLogsResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_logs(
        chain_id, block_from, block_to, topics0, topics1, topics2, topics3, addresses, None, false,
    )
    .await
    .map_err(|e| format!("failed to read logs: {}", e))
}

#[update]
pub async fn read_logs_with_proof(
    chain_id: u64,
    block_from: Option<u64>,
    block_to: Option<u64>,
    topics0: Option<Vec<String>>,
    topics1: Option<Vec<String>>,
    topics2: Option<Vec<String>>,
    topics3: Option<Vec<String>>,
    addresses: Option<Vec<String>>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadLogsResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_logs(
        chain_id, block_from, block_to, topics0, topics1, topics2, topics3, addresses, None, true,
    )
    .await
    .map_err(|e| format!("failed to read logs: {}", e))
}

#[inline]
pub async fn _read_logs(
    chain_id: u64,
    block_from: Option<u64>,
    block_to: Option<u64>,
    topics0: Option<Vec<String>>,
    topics1: Option<Vec<String>>,
    topics2: Option<Vec<String>>,
    topics3: Option<Vec<String>>,
    addresses: Option<Vec<String>>,
    payer: Option<String>,
    with_signature: bool,
) -> Result<ReadLogsResult, CustomFeedError> {
    let base_fee = state::get_cfg().balances_cfg.base_fee;

    if let Some(ref payer) = payer {
        if !Balances::is_sufficient(payer, &base_fee)? {
            return Err(BalanceError::InsufficientBalance)?;
        };
    }

    let chain_rpc = ChainsRPC::get_first_chain_rpc(chain_id)?;

    let w3 = web3::instance(chain_rpc, clone_with_state!(evm_rpc_canister));

    let logs = w3
        .get_logs(
            block_from,
            block_to,
            topics0
                .clone()
                .map(|v| v.into_iter().map(|t| H256::from_str(&t).unwrap()).collect()),
            topics1
                .clone()
                .map(|v| v.into_iter().map(|t| H256::from_str(&t).unwrap()).collect()),
            topics2
                .clone()
                .map(|v| v.into_iter().map(|t| H256::from_str(&t).unwrap()).collect()),
            topics3
                .clone()
                .map(|v| v.into_iter().map(|t| H256::from_str(&t).unwrap()).collect()),
            addresses.clone().map(|v| {
                v.into_iter()
                    .map(|t| address::to_h160(&t).unwrap())
                    .collect()
            }),
        )
        .await?;

    let mut result = ReadLogsResult {
        data: logs.into_iter().map(ReadLogsData::from).collect(),
        meta: ReadLogsMetadata {
            chain_id,
            block_from: block_from.unwrap_or_default(),
            block_to: block_to.unwrap_or_default(),
            topics0: topics0.unwrap_or_default(),
            topics1: topics1.unwrap_or_default(),
            topics2: topics2.unwrap_or_default(),
            topics3: topics3.unwrap_or_default(),
            addresses: addresses.unwrap_or_default(),
            timestamp: in_seconds(),
        },
        signature: None,
    };

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &base_fee)?;
        Balances::add_amount(&canister::eth_address().await?, &base_fee)?;
    }

    Ok(result)
}

pub async fn _get_asset_data(
    id: String,
    with_signature: bool,
    payer: Option<String>,
) -> Result<AssetDataResult, AssetsError> {
    if with_signature {
        metrics!(inc GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc GET_ASSET_DATA_CALLS, id);
    }
    let rate = FeedStorage::rate(&id, with_signature, payer).await?;

    if with_signature {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS, id);
    } else {
        metrics!(inc SUCCESSFUL_GET_ASSET_DATA_CALLS, id);
    }
    Ok(rate)
}

#[update]
pub async fn get_xrc_data(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_xrc_data(id, false, Some(payer))
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[update]
pub async fn get_xrc_data_with_proof(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_xrc_data(id, true, Some(payer))
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

pub async fn _get_xrc_data(
    id: String,
    with_signature: bool,
    payer: Option<String>,
) -> Result<AssetDataResult, AssetsError> {
    let rate = Feed {
        id: id.clone(),
        update_freq: DEFAULT_UPDATE_FREQ,
        ..Default::default()
    };

    let mut rate = FeedStorage::get_default_rate(&rate, None).await?;

    if with_signature {
        rate.sign().await.map_err(FeedError::RateDataError)?;
    }

    Ok(rate)
}

#[update]
pub async fn get_asset_data(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<AssetDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_asset_data(id, false, None)
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[update]
pub async fn get_multiple_assets_data_with_proof(
    ids: Vec<String>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<MultipleAssetsDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let mut multiple_assetds_data = _get_multiple_assets_data(ids, true, None)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))?;

    multiple_assetds_data
        .sign()
        .await
        .map_err(|e| format!("failed to sign: {}", e))?;

    Ok(multiple_assetds_data)
}

#[update]
pub async fn get_multiple_assets_data(
    ids: Vec<String>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<MultipleAssetsDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _get_multiple_assets_data(ids, false, None)
        .await
        .map_err(|e| format!("failed to get assets data: {}", e))
}

pub async fn _get_multiple_assets_data(
    ids: Vec<String>,
    with_signature: bool,
    payer: Option<String>,
) -> Result<MultipleAssetsDataResult, AssetsError> {
    let mut data = Vec::with_capacity(ids.len());

    let futures = ids
        .into_iter()
        .map(|id| _get_asset_data(id, false, payer.clone()))
        .collect::<Vec<_>>();

    for result in join_all(futures).await {
        data.push(result?.data);
    }

    let mut rates = MultipleAssetsDataResult {
        data,
        signature: None,
    };

    if with_signature {
        rates.sign().await.map_err(FeedError::RateDataError)?;
    }

    Ok(rates)
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
