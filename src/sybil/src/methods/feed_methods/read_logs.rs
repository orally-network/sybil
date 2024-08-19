use std::str::FromStr;

use ic_cdk::update;

use ic_web3_rs::types::H256;
use sybil_utils::cycles_count;

use crate::{
    clone_with_state, log,
    methods::{balances, custom_feeds::CustomFeedError},
    stringify_func_call,
    types::{
        balances::{BalanceError, Balances},
        cache::Cache,
        chains_rpc::ChainsRPC,
        feed_types::read_logs::{ReadLogsData, ReadLogsMetadata, ReadLogsResult},
        state,
    },
    utils::{address, canister, convertion::convert_usd_to_eth, siwe, time::in_seconds, web3},
};

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

    let func_signature = stringify_func_call!(_read_logs(
        chain_id, block_from, block_to, topics0, topics1, topics2, topics3, addresses, false
    ));

    let cache_builder = Cache::with(
        func_signature,
        _read_logs(
            chain_id,
            block_from,
            block_to,
            topics0,
            topics1,
            topics2,
            topics3,
            addresses,
            Some(payer),
            false,
        ),
    );

    cache_builder
        .evaluate()
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

    let func_signature = stringify_func_call!(_read_logs(
        chain_id, block_from, block_to, topics0, topics1, topics2, topics3, addresses, true
    ));

    let cache_builder = Cache::with(
        func_signature,
        _read_logs(
            chain_id,
            block_from,
            block_to,
            topics0,
            topics1,
            topics2,
            topics3,
            addresses,
            Some(payer),
            true,
        ),
    );

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to read logs: {}", e))
}

#[inline]
#[cycles_count]
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
        meta: Some(ReadLogsMetadata {
            chain_id,
            block_from: block_from.unwrap_or_default(),
            block_to: block_to.unwrap_or_default(),
            topics0: topics0.unwrap_or_default(),
            topics1: topics1.unwrap_or_default(),
            topics2: topics2.unwrap_or_default(),
            topics3: topics3.unwrap_or_default(),
            addresses: addresses.unwrap_or_default(),
            timestamp: in_seconds(),
            fee: 0.into(),
            fee_symbol: "ETH".to_string(), // TODO: it's hardcoded, change it properly
        }),
        signature: None,
        bytes: None,
    };

    if payer.is_none() {
        let fee = convert_usd_to_eth(base_fee.clone(), balances::DECIMALS).await?;
        result.meta.as_mut().map(|meta| meta.fee = fee);
    }

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &base_fee)?;
        Balances::add_amount(&canister::eth_address().await?, &base_fee)?;
    }

    Ok(result)
}
