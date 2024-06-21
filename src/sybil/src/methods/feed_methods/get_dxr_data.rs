use std::{
    cmp::max,
    fmt::{Display, Formatter},
};

use candid::CandidType;
use ethers_core::abi::Token;
use ic_cdk::update;
use ic_web3_rs::{contract::Contract, types::U256};
use serde::{Deserialize, Serialize};
use sybil_utils::cycles_count;

use crate::{
    clone_with_state, log,
    methods::{balances, custom_feeds::CustomFeedError},
    stringify_func_call,
    types::{
        balances::Balances,
        cache::Cache,
        chains_rpc::ChainsRPC,
        feed_types::get_dxr_data::{GetDXRData, GetDXRDataMetadata, GetDXRDataResult},
        state,
    },
    utils::{address, canister, convertion::convert_usd_to_eth, siwe, time::in_seconds, web3},
};

#[derive(CandidType, Debug, Deserialize, Serialize)]
pub enum DexType {
    UniswapV2,
}

impl Display for DexType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DexType::UniswapV2 => write!(f, "UniswapV2"),
        }
    }
}

const UNISWAP_V2_PAIR_ABI: &[u8] = include_bytes!("../../../../../assets/UniswapV2PairABI.json");
const ERC20_ABI: &[u8] = include_bytes!("../../../../../assets/ERC20ABI.json");

const TARGET_DECIMALS: u32 = 9;

const GET_RESERVES_FUNCTION_NAME: &str = "getReserves";
const TOKEN0_FUNCTION_NAME: &str = "token0";
const TOKEN1_FUNCTION_NAME: &str = "token1";
const DECIMALS_FUNCTION_NAME: &str = "decimals";

#[update]
pub async fn get_dxr_data(
    chain_id: u64,
    pool_address: String,
    block_numbers: Option<Vec<u64>>,
    dex_type: DexType,
    reverse_pair: Option<bool>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<GetDXRDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let func_signature = stringify_func_call!(_get_dxr_data(
        chain_id,
        pool_address,
        block_numbers,
        dex_type,
        reverse_pair,
        false
    ));

    let cache_builder = Cache::with(
        func_signature,
        _get_dxr_data(
            chain_id,
            pool_address,
            block_numbers,
            dex_type,
            reverse_pair,
            Some(payer),
            false,
        ),
    );

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to get dxr data: {}", e))
}

#[update]
pub async fn get_dxr_data_with_proof(
    chain_id: u64,
    pool_address: String,
    block_numbers: Option<Vec<u64>>,
    dex_type: DexType,
    reverse_pair: Option<bool>,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<GetDXRDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let func_signature = stringify_func_call!(_get_dxr_data(
        chain_id,
        pool_address,
        block_numbers,
        dex_type,
        reverse_pair,
        true
    ));

    let cache_builder = Cache::with(
        func_signature,
        _get_dxr_data(
            chain_id,
            pool_address,
            block_numbers,
            dex_type,
            reverse_pair,
            Some(payer),
            true,
        ),
    );

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to get dxr data: {}", e))
}

#[inline]
#[cycles_count]
pub async fn _get_dxr_data(
    chain_id: u64,
    pool_address: String,
    block_numbers: Option<Vec<u64>>,
    dex_type: DexType,
    reverse_pair: Option<bool>,
    payer: Option<String>,
    with_signature: bool,
) -> Result<GetDXRDataResult, CustomFeedError> {
    let chain_rpc = ChainsRPC::get_first_chain_rpc(chain_id).unwrap();

    let w3 = web3::batch_instance(chain_rpc, clone_with_state!(evm_rpc_canister));

    let contract_address = address::to_h160(&pool_address)?;

    let uniswap_v2_pair_contract =
        Contract::from_json(w3.eth(), contract_address, UNISWAP_V2_PAIR_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?;

    let tokens = vec![];

    let from = address::to_h160(&canister::eth_address().await?.to_string())?;

    // Getting the token0 and token1 addresses
    let token0_name = w3.get_call_result_promise(
        &uniswap_v2_pair_contract,
        &TOKEN0_FUNCTION_NAME,
        &tokens,
        from,
        Some(contract_address),
        None,
    );

    let token1_name = w3.get_call_result_promise(
        &uniswap_v2_pair_contract,
        &TOKEN1_FUNCTION_NAME,
        &tokens,
        from,
        Some(contract_address),
        None,
    );

    // Getting the reserves for each provided block number.
    // If no block number is provided, we get the reserves for the latest block
    let result = if let Some(ref block_nubmers) = block_numbers {
        let futures = block_nubmers.iter().map(|block_number| {
            w3.get_call_result_promise(
                &uniswap_v2_pair_contract,
                &GET_RESERVES_FUNCTION_NAME,
                &tokens,
                from,
                Some(contract_address),
                Some((*block_number).into()),
            )
        });

        w3.submit_batch().await?;
        let mut results = Vec::with_capacity(futures.len());
        for result in futures {
            results.push(result.await?);
        }

        results
    } else {
        let call_result = w3.get_call_result_promise(
            &uniswap_v2_pair_contract,
            &GET_RESERVES_FUNCTION_NAME,
            &tokens,
            from,
            Some(contract_address),
            None,
        );

        w3.submit_batch().await?;

        vec![call_result.await?]
    };

    // Getting the token0 and token1 addresses
    let token0_contract_addr = token0_name.await?.pop().unwrap().into_address().unwrap();
    let token1_contract_addr = token1_name.await?.pop().unwrap().into_address().unwrap();

    let token0_contract = Contract::from_json(w3.eth(), token0_contract_addr, ERC20_ABI)
        .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?;

    let token1_contract = Contract::from_json(w3.eth(), token1_contract_addr, ERC20_ABI)
        .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?;

    // Getting the decimals for each token
    let token0_decimals = w3.get_call_result_promise(
        &token0_contract,
        &DECIMALS_FUNCTION_NAME,
        &tokens,
        from,
        Some(token0_contract_addr),
        None,
    );

    let token1_decimals = w3.get_call_result_promise(
        &token1_contract,
        &DECIMALS_FUNCTION_NAME,
        &tokens,
        from,
        Some(token1_contract_addr),
        None,
    );

    w3.submit_batch().await?;

    let mut token0_decimals = token0_decimals
        .await?
        .pop()
        .unwrap()
        .into_uint()
        .unwrap()
        .as_u32();
    let mut token1_decimals = token1_decimals
        .await?
        .pop()
        .unwrap()
        .into_uint()
        .unwrap()
        .as_u32();

    let mut reserve0 = U256::zero();
    let mut reserve1 = U256::zero();
    let mut timestamp = 0;

    // Calculating the average rate
    result.clone().into_iter().for_each(|reserve| {
        let reserve0_token: Token = reserve[0].clone().into();
        let reserve1_token: Token = reserve[1].clone().into();

        reserve0 += reserve0_token.into_uint().unwrap();
        reserve1 += reserve1_token.into_uint().unwrap();
        timestamp = max(timestamp, reserve[2].clone().into_uint().unwrap().as_u64())
    });

    // If the reverse_pair is true, we swap the reserves and the decimals
    if reverse_pair.unwrap_or(false) {
        std::mem::swap(&mut reserve0, &mut reserve1);
        std::mem::swap(&mut token0_decimals, &mut token1_decimals);
    }

    // Adding 9 zeros to the reserve0 wich represents the amount of decimals we want to have
    reserve0 *= U256::from(10).pow(U256::from(TARGET_DECIMALS));

    // Adjusting the decimals
    if token0_decimals > token1_decimals {
        reserve0 /= U256::from(10).pow(U256::from(token0_decimals - token1_decimals));
    } else {
        reserve0 *= U256::from(10).pow(U256::from(token1_decimals - token0_decimals));
    }

    let rate = reserve0 / reserve1;

    let mut result = GetDXRDataResult {
        data: GetDXRData {
            feed_id: format!("UniswapV2Pool-{}", pool_address),
            rate: rate.as_u64(),
            decimals: TARGET_DECIMALS as u64,
            timestamp,
        },
        meta: GetDXRDataMetadata {
            chain_id,
            pool_address,
            block_numbers: block_numbers.unwrap_or_default(),
            dex_type: dex_type.to_string(),
            reverse_pair: reverse_pair.unwrap_or(false),
            fee: 0.into(),
            fee_symbol: "ETH".to_string(),
            timestamp: in_seconds(),
        },
        signature: None,
    };

    let cost = state::get_cfg().balances_cfg.base_fee.clone();

    if payer.is_none() {
        let fee = convert_usd_to_eth(cost.clone(), balances::DECIMALS).await?;
        result.meta.fee = fee;
    }

    if with_signature {
        result.sign().await?;
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &cost)?;
        Balances::add_amount(&canister::eth_address().await?, &cost)?;
    }

    Ok(result)
}
