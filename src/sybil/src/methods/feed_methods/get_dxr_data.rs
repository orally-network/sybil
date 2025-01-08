use std::{future::Future, str::FromStr, sync::Arc};

use candid::Nat;
use ethers_core::abi::Token;
use futures::future::try_join_all;
use ic_cdk::update;
use ic_web3_rs::{
    contract::Contract,
    transports::Batch,
    types::{H160, U256},
    BatchTransport, Transport,
};
use sybil_utils::cycles_count;

use crate::{
    clone_with_state, log,
    methods::{balances, custom_feeds::CustomFeedError},
    stringify_func_call,
    types::{
        balances::Balances,
        cache::Cache,
        chains_rpc::ChainsRPC,
        dex_list::{DEXList, DEX},
        feed_types::get_dxr_data::{
            Aggregation, DexType, GetDXRData, GetDXRDataBatchResult, GetDXRDataMetadata,
            GetDXRDataResult,
        },
        state,
    },
    utils::{
        address::{self, AddressError},
        canister,
        convertion::convert_usd_to_eth,
        siwe,
        time::in_seconds,
        web3::{
            self,
            utils::{
                get_block_response_len, get_decimals_and_symbols_batch_response_len,
                get_reserves_batch_response_len, get_token_address_batch_response_len,
            },
            Web3Instance,
        },
    },
};

const UNISWAP_V2_PAIR_ABI: &[u8] = include_bytes!("../../../../../assets/UniswapV2PairABI.json");
const ERC20_ABI: &[u8] = include_bytes!("../../../../../assets/ERC20ABI.json");

const TARGET_DECIMALS: u32 = 9;

const GET_RESERVES_FUNCTION_NAME: &str = "getReserves";
const PRICE0_CUMULATIVE_LAST_FUNCTION_NAME: &str = "price0CumulativeLast";
const PRICE1_CUMULATIVE_LAST_FUNCTION_NAME: &str = "price1CumulativeLast";
const TOKEN0_FUNCTION_NAME: &str = "token0";
const TOKEN1_FUNCTION_NAME: &str = "token1";
const DECIMALS_FUNCTION_NAME: &str = "decimals";
const SYMBOL_FUNCTION_NAME: &str = "symbol";

#[update]
pub async fn get_dxr_data(
    chain_id: u64,
    pool_address: String,
    aggregation: Option<Aggregation>,
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
        aggregation,
        dex_type,
        reverse_pair,
        false,
        true
    ));

    let cache_builder = Cache::with(
        func_signature,
        _get_dxr_data(
            chain_id,
            pool_address,
            aggregation,
            dex_type,
            reverse_pair,
            false,
            true,
            Some(payer),
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
    aggregation: Option<Aggregation>,
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
        aggregation,
        dex_type,
        reverse_pair,
        true,
        true
    ));

    let cache_builder = Cache::with(
        func_signature,
        _get_dxr_data(
            chain_id,
            pool_address,
            aggregation,
            dex_type,
            reverse_pair,
            true,
            true,
            Some(payer),
        ),
    );

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to get dxr data: {}", e))
}

async fn get_dex<T: Transport + 'static>(
    chain_id: u64,
    contract: Arc<Contract<T>>,
    pool_address: String,
    _dex_type: DexType,
) -> Result<DEX, CustomFeedError> {
    if let Some(dex) = DEXList::get_dex(&pool_address) {
        return Ok(dex);
    }

    let chain_rpc = ChainsRPC::get_chain_rpc(chain_id)?;

    let w3 = web3::batch_instance(chain_id, Some(chain_rpc), clone_with_state!(evm_rpc_canister), None);

    let tokens = vec![];
    let contract_address = address::to_h160(&pool_address)?;
    let from = address::to_h160(&canister::eth_address().await?.to_string())?;

    let token0_contract_addr = w3.get_call_result_promise(
        contract.clone(),
        TOKEN0_FUNCTION_NAME,
        &tokens,
        from,
        Some(contract_address),
        None,
    )?;

    let token1_contract_addr = w3.get_call_result_promise(
        contract.clone(),
        TOKEN1_FUNCTION_NAME,
        &tokens,
        from,
        Some(contract_address),
        None,
    )?;

    w3.submit_batch().await?;

    let token0_contract_addr = token0_contract_addr
        .await
        .unwrap()?
        .pop()
        .unwrap()
        .into_address()
        .unwrap();
    let token1_contract_addr = token1_contract_addr
        .await
        .unwrap()?
        .pop()
        .unwrap()
        .into_address()
        .unwrap();

    let token0_contract = Arc::new(
        Contract::from_json(w3.eth(), token0_contract_addr, ERC20_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    let token1_contract = Arc::new(
        Contract::from_json(w3.eth(), token1_contract_addr, ERC20_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    // Getting the decimals for each token
    let token0_decimals = w3.get_call_result_promise(
        token0_contract.clone(),
        &DECIMALS_FUNCTION_NAME,
        &tokens,
        from,
        Some(token0_contract_addr),
        None,
    )?;

    let token1_decimals = w3.get_call_result_promise(
        token1_contract.clone(),
        DECIMALS_FUNCTION_NAME,
        &tokens,
        from,
        Some(token1_contract_addr),
        None,
    )?;

    let token0_symbol = w3.get_call_result_promise(
        token0_contract.clone(),
        SYMBOL_FUNCTION_NAME,
        &tokens,
        from,
        Some(token0_contract_addr),
        None,
    )?;

    let token1_symbol = w3.get_call_result_promise(
        token1_contract.clone(),
        SYMBOL_FUNCTION_NAME,
        &tokens,
        from,
        Some(token1_contract_addr),
        None,
    )?;

    w3.submit_batch().await?;

    let token0_decimals = token0_decimals
        .await
        .unwrap()?
        .pop()
        .unwrap()
        .into_uint()
        .unwrap()
        .as_u32();

    let token1_decimals = token1_decimals
        .await
        .unwrap()?
        .pop()
        .unwrap()
        .into_uint()
        .unwrap()
        .as_u32();

    let token0_symbol = token0_symbol.await.unwrap()?.pop().unwrap();

    let token1_symbol = token1_symbol.await.unwrap()?.pop().unwrap();

    let token0_symbol = match token0_symbol {
        Token::String(s) => s,
        // Each byte must be greater than 0, because this token represents utf8 string in bytes32.
        // Thus, we can safely trim the trailing zeros and convert remaining bytes into a string.
        Token::FixedBytes(b) | Token::Bytes(b) => {
            let bytes = b.into_iter().take_while(|&c| c != 0).collect::<Vec<_>>();
            match String::from_utf8(bytes) {
                Ok(s) => s,
                _ => {
                    return Err(CustomFeedError::FailedToParseABI(
                        "Failed to parse token0 symbol".to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(CustomFeedError::FailedToParseABI(
                "Failed to parse token0 symbol".to_string(),
            ))
        }
    };

    let token1_symbol = match token1_symbol {
        Token::String(s) => s,
        // Each byte must be greater than 0, because this token represents utf8 string in bytes32.
        // Thus, we can safely trim the trailing zeros and convert remaining bytes into a string.
        Token::FixedBytes(b) | Token::Bytes(b) => {
            let bytes = b.into_iter().take_while(|&c| c != 0).collect::<Vec<_>>();
            match String::from_utf8(bytes) {
                Ok(s) => s,
                _ => {
                    return Err(CustomFeedError::FailedToParseABI(
                        "Failed to parse token1 symbol".to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(CustomFeedError::FailedToParseABI(
                "Failed to parse token1 symbol".to_string(),
            ))
        }
    };

    let dex = DEX {
        address: pool_address.clone(),
        token0_address: format!("{:?}", token0_contract_addr),
        token0_symbol,
        token0_decimals,
        token1_address: format!("{:?}", token1_contract_addr),
        token1_symbol,
        token1_decimals,
    };

    DEXList::add_dex(pool_address, dex.clone());

    Ok(dex)
}

#[inline]
#[cycles_count]
pub async fn _get_dxr_data_batch(
    chain_id: u64,
    pool_addresses: &[String],
    _dex_type: DexType,
    reverse_pair: Option<bool>,
    with_signature: bool,
    with_meta: bool,
    payer: Option<String>,
) -> Result<GetDXRDataBatchResult, CustomFeedError> {
    let chain_rpc = ChainsRPC::get_chain_rpc(chain_id)?;

    // w3 optimized for getting the block number
    let w3_block = web3::instance(
        chain_id,
        Some(chain_rpc.clone()),
        clone_with_state!(evm_rpc_canister),
        Some(get_block_response_len()),
    );

    let balance_before = ic_cdk::api::canister_balance();

    let first_chain_rpc = ChainsRPC::get_first_chain_rpc(chain_id)?;
    let range_len = first_chain_rpc.config.num_of_blocks_for_get_dxr_data;
    let last_block = w3_block.get_block().await?;
    let range = (last_block - range_len, last_block);

    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for preparing _get_dxr_data_batch: {}",
        balance_before - balance_after
    );

    let mut futures = Vec::with_capacity(pool_addresses.len());

    let balance_before = ic_cdk::api::canister_balance();
    let time_before = ic_cdk::api::time() / 1_000_000;
    let dexes = get_dexes(chain_rpc.clone(), pool_addresses, _dex_type).await?;
    let time_after = ic_cdk::api::time() / 1_000_000;
    let balance_after = ic_cdk::api::canister_balance();
    log!("cost for getting dexes: {}", balance_before - balance_after);
    log!(
        "time for getting dexes: {}",
        (time_after - time_before) as f64 / 1_000.0
    );

    let timestamp = in_seconds();
    let from = address::to_h160(&canister::eth_address().await?.to_string())?;

    // w3 optimized for getting the reserves
    let w3 = Arc::new(web3::batch_instance(
        chain_id,
        Some(chain_rpc.clone()),
        clone_with_state!(evm_rpc_canister),
        Some(get_reserves_batch_response_len(dexes.len() as u64)),
    ));

    for dex in dexes {
        futures.push(get_dxr_data_for_dex(
            w3.clone(),
            dex,
            range,
            reverse_pair,
            timestamp,
            from,
        ));
    }

    let time_before = ic_cdk::api::time() / 1_000_000;
    let balance_before = ic_cdk::api::canister_balance();
    w3.submit_batch().await?;
    let data = try_join_all(futures).await?;
    let balance_after = ic_cdk::api::canister_balance();
    log!("cost for getting data: {}", balance_before - balance_after);
    let time_after = ic_cdk::api::time() / 1_000_000;
    log!(
        "time for getting data: {}",
        (time_after - time_before) as f64 / 1_000.0
    );

    log!("DATA SIZE: {}", data.len());

    let mut result = GetDXRDataBatchResult {
        data,
        meta: if with_meta {
            Some(GetDXRDataMetadata {
                // chain_id,
                // pool_address,
                // block_numbers: block_numbers.unwrap_or_default(),
                // dex_type: dex_type.to_string(),
                // reverse_pair: reverse_pair.unwrap_or(false),
                timestamp: in_seconds(),
                fee: 0.into(),
                fee_symbol: "ETH".to_string(),
            })
        } else {
            None
        },
        signature: None,
        bytes: None,
    };

    let cost = state::get_cfg().balances_cfg.base_fee.clone() * Nat::from(pool_addresses.len() * 4);
    let signature_fee = state::get_cfg().balances_cfg.signature_fee.clone();

    if payer.is_none() {
        let fee = convert_usd_to_eth(cost.clone(), balances::DECIMALS).await?;
        result.meta.as_mut().map(|meta| meta.fee = fee);
    }

    if with_signature {
        let time_before = ic_cdk::api::time() / 1_000_000;
        let balance_before = ic_cdk::api::canister_balance();
        result.sign().await?;
        let balance_after = ic_cdk::api::canister_balance();
        let time_after = ic_cdk::api::time() / 1_000_000;
        log!(
            "cost for signing _get_dxr_data_batch: {}",
            balance_before - balance_after
        );
        log!(
            "time for signing: {}",
            (time_after - time_before) as f64 / 1_000.0
        );

        if let Some(payer) = &payer {
            Balances::reduce_amount(payer, &signature_fee)?;
            Balances::add_amount(&canister::eth_address().await?, &signature_fee)?;
        }
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &cost)?;
        Balances::add_amount(&canister::eth_address().await?, &cost)?;
    }

    Ok(result)
}

#[inline]
#[cycles_count]
pub async fn _get_dxr_data(
    chain_id: u64,
    pool_address: String,
    aggregation: Option<Aggregation>,
    _dex_type: DexType,
    reverse_pair: Option<bool>,
    with_signature: bool,
    with_meta: bool,
    payer: Option<String>,
) -> Result<GetDXRDataResult, CustomFeedError> {
    let balance_before = ic_cdk::api::canister_balance();
    let chain_rpc = ChainsRPC::get_chain_rpc(chain_id)?;

    let w3 = web3::batch_instance(chain_id, Some(chain_rpc), clone_with_state!(evm_rpc_canister), None);

    let contract_address = address::to_h160(&pool_address)?;

    let uniswap_v2_pair_contract = Arc::new(
        Contract::from_json(w3.eth(), contract_address, UNISWAP_V2_PAIR_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    let tokens = vec![];

    let from = address::to_h160(&canister::eth_address().await?.to_string())?;

    let DEX {
        mut token0_decimals,
        mut token1_decimals,
        mut token0_symbol,
        mut token1_symbol,
        ..
    } = get_dex(
        chain_id,
        uniswap_v2_pair_contract.clone(),
        pool_address.clone(),
        _dex_type,
    )
    .await?;

    let block_numbers: Vec<u64> = match aggregation.unwrap_or(Aggregation::AvgFromLastBlocks(1)) {
        Aggregation::AvgFromRange(block_numbers) => {
            (block_numbers.0..block_numbers.1).collect()
        }
        Aggregation::AvgFromLastBlocks(last_blocks) => {
            let block_number_promise = w3.get_block_promise();
            w3.submit_batch().await?;
            let block = block_number_promise.await.unwrap().as_u64();

            (block - last_blocks..block).collect()
        }
    };

    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for preparing _get_dxr_data: {}",
        balance_before - balance_after
    );
    // Getting the reserves for each provided block number.
    // If no block number is provided, we get the reserves for the latest block
    let balance_before = ic_cdk::api::canister_balance();
    let futures = block_numbers
        .iter()
        .map(|block_number| {
            w3.get_call_result_promise(
                uniswap_v2_pair_contract.clone(),
                GET_RESERVES_FUNCTION_NAME,
                &tokens,
                from,
                Some(contract_address),
                Some((*block_number).into()),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    w3.submit_batch().await?;
    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for getting reserves: {}",
        balance_before - balance_after
    );
    let balance_before = ic_cdk::api::canister_balance();
    let mut results = Vec::with_capacity(futures.len());

    for result in futures {
        results.push(result.await.unwrap()?);
    }

    let mut reserve0 = U256::zero();
    let mut reserve1 = U256::zero();
    let timestamp = in_seconds();

    // Calculating the average rate
    results.clone().into_iter().for_each(|reserve| {
        let reserve0_token: Token = reserve[0].clone();
        let reserve1_token: Token = reserve[1].clone();

        reserve0 += reserve0_token.into_uint().unwrap();
        reserve1 += reserve1_token.into_uint().unwrap();
    });

    // If the reverse_pair is true, we swap the reserves and the decimals
    if reverse_pair.unwrap_or(false) {
        std::mem::swap(&mut reserve0, &mut reserve1);
        std::mem::swap(&mut token0_decimals, &mut token1_decimals);
        std::mem::swap(&mut token0_symbol, &mut token1_symbol);
    }

    // Adding 9 zeros to the reserve1 wich represents the amount of decimals we want to have
    reserve1 *= U256::from(10).pow(U256::from(TARGET_DECIMALS));

    // Adjusting the decimals
    if token1_decimals > token0_decimals {
        reserve1 /= U256::from(10).pow(U256::from(token1_decimals - token0_decimals));
    } else {
        reserve1 *= U256::from(10).pow(U256::from(token0_decimals - token1_decimals));
    }

    let rate = reserve1 / reserve0;

    let mut result = GetDXRDataResult {
        data: GetDXRData {
            feed_id: format!(
                "UniswapV2Pool-{}-{}/{}",
                pool_address, token0_symbol, token1_symbol
            ),
            rate: rate.as_u64(),
            decimals: TARGET_DECIMALS as u64,
            timestamp,
        },
        meta: if with_meta {
            Some(GetDXRDataMetadata {
                // chain_id,
                // pool_address,
                // block_numbers: block_numbers.unwrap_or_default(),
                // dex_type: dex_type.to_string(),
                // reverse_pair: reverse_pair.unwrap_or(false),
                timestamp: in_seconds(),
                fee: 0.into(),
                fee_symbol: "ETH".to_string(),
            })
        } else {
            None
        },
        signature: None,
        bytes: None,
    };

    let cost = state::get_cfg().balances_cfg.base_fee.clone() * Nat::from(block_numbers.len());
    let signature_fee = state::get_cfg().balances_cfg.signature_fee.clone();

    if payer.is_none() {
        let fee = convert_usd_to_eth(cost.clone(), balances::DECIMALS).await?;
        result.meta.as_mut().map(|meta| meta.fee = fee);
    }

    if with_signature {
        result.sign().await?;

        if let Some(payer) = &payer {
            Balances::reduce_amount(payer, &signature_fee)?;
            Balances::add_amount(&canister::eth_address().await?, &signature_fee)?;
        }
    }

    if let Some(payer) = payer {
        Balances::reduce_amount(&payer, &cost)?;
        Balances::add_amount(&canister::eth_address().await?, &cost)?;
    }

    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for casting and returning answer: {}",
        balance_before - balance_after
    );
    Ok(result)
}

fn get_dxr_data_for_dex<B: BatchTransport + 'static>(
    w3: Arc<Web3Instance<Batch<B>>>,
    dex: DEX,
    range: (u64, u64),
    reverse_pair: Option<bool>,
    sybil_timestamp: u64,
    from: H160,
) -> impl Future<Output = Result<GetDXRData, CustomFeedError>> {
    let tokens = vec![];

    let DEX {
        address,
        mut token0_decimals,
        mut token1_decimals,
        mut token0_symbol,
        mut token1_symbol,
        ..
    } = dex;

    let contract_address = address::to_h160(&address).unwrap();

    let contract = Arc::new(
        Contract::from_json(w3.eth(), contract_address, UNISWAP_V2_PAIR_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))
            .unwrap(),
    );

    let first_block_number = range.0;
    let last_block_number = range.1;

    let cumulative_function_name = if reverse_pair.unwrap_or(false) {
        PRICE1_CUMULATIVE_LAST_FUNCTION_NAME
    } else {
        PRICE0_CUMULATIVE_LAST_FUNCTION_NAME
    };

    let price1_cumulative = w3
        .get_call_result_promise(
            contract.clone(),
            cumulative_function_name,
            &tokens,
            from,
            Some(contract_address),
            Some(last_block_number.into()),
        )
        .unwrap();

    let price1_cumulative_last = w3
        .get_call_result_promise(
            contract.clone(),
            cumulative_function_name,
            &tokens,
            from,
            Some(contract_address),
            Some(first_block_number.into()),
        )
        .unwrap();

    let reserves_last = w3
        .get_call_result_promise(
            contract.clone(),
            GET_RESERVES_FUNCTION_NAME,
            &tokens,
            from,
            Some(contract_address),
            Some(first_block_number.into()),
        )
        .unwrap();

    let reserves = w3
        .get_call_result_promise(
            contract.clone(),
            GET_RESERVES_FUNCTION_NAME,
            &tokens,
            from,
            Some(contract_address),
            Some(last_block_number.into()),
        )
        .unwrap();

    async move {
        let reserves_last = reserves_last.await.unwrap()?;

        let mut reserve0_last = reserves_last[0]
            .clone()
            .into_uint()
            .expect("reserve0_last should be an uint");
        let mut reserve1_last = reserves_last[1]
            .clone()
            .into_uint()
            .expect("reserve1_last should be an uint");
        let timestamp_last = reserves_last[2]
            .clone()
            .into_uint()
            .expect("timestamp_last should be an uint");

        let reserves = reserves.await.unwrap()?;

        let timestamp = reserves[2]
            .clone()
            .into_uint()
            .expect("timestamp should be an uint");

        let price = if timestamp == timestamp_last {
            // If the reverse_pair is true, we swap the reserves and the decimals
            if reverse_pair.unwrap_or(false) {
                std::mem::swap(&mut reserve0_last, &mut reserve1_last);
                std::mem::swap(&mut token0_decimals, &mut token1_decimals);
                std::mem::swap(&mut token0_symbol, &mut token1_symbol);
            }

            // Adding 9 zeros to the reserve1 wich represents the amount of decimals we want to have
            reserve1_last *= U256::from(10).pow(U256::from(TARGET_DECIMALS));

            // Adjusting the decimals
            if token1_decimals > token0_decimals {
                reserve1_last /= U256::from(10).pow(U256::from(token1_decimals - token0_decimals));
            } else {
                reserve1_last *= U256::from(10).pow(U256::from(token0_decimals - token1_decimals));
            }

            reserve1_last / reserve0_last // price
        } else {
            let price1_cumulative = price1_cumulative
                .await
                .unwrap()?
                .first()
                .cloned()
                .unwrap()
                .into_uint()
                .expect("price1Cumulative should be an uint");
            let price1_cumulative_last = price1_cumulative_last
                .await
                .unwrap()?
                .first()
                .cloned()
                .unwrap()
                .into_uint()
                .expect("price1CumulativeLast should be an uint");

            // price1_cumulative is not U256, but [UQ112x112](https://github.com/Uniswap/v2-periphery/blob/6d03bede0a97c72323fa1c379ed3fdf7231d0b26/contracts/examples/ExampleOracleSimple.sol#L50)
            let price = (price1_cumulative - price1_cumulative_last) / (timestamp - timestamp_last);
            let price = price * U256::from(10).pow(U256::from(TARGET_DECIMALS));

            price >> 112
        };

        let data = GetDXRData {
            feed_id: format!(
                "UniswapV2Pool-{}-{}/{}",
                address, token0_symbol, token1_symbol
            ),
            rate: price.as_u64(),
            decimals: TARGET_DECIMALS as u64,
            timestamp: sybil_timestamp,
        };

        Ok(data)
    }
}

fn get_dex_with_token_addresses_and_pool<B: BatchTransport + 'static>(
    w3: Arc<Web3Instance<Batch<B>>>,
    pool_address: &str,
    _dex_type: DexType,
    from: H160,
) -> Result<impl Future<Output = Result<DEX, CustomFeedError>> + '_, CustomFeedError> {
    let contract_address = address::to_h160(pool_address)?;

    let contract = Arc::new(
        Contract::from_json(w3.eth(), contract_address, UNISWAP_V2_PAIR_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    let tokens = vec![];
    let contract_address = address::to_h160(pool_address)?;

    let dex = DEXList::get_dex(pool_address);

    let mut token0_contract_addr = None;
    let mut token1_contract_addr = None;

    // If the dex is not in the list, we need to get the token addresses
    if dex.is_none() {
        token0_contract_addr = Some(w3.get_call_result_promise(
            contract.clone(),
            TOKEN0_FUNCTION_NAME,
            &tokens,
            from,
            Some(contract_address),
            None,
        )?);

        token1_contract_addr = Some(w3.get_call_result_promise(
            contract.clone(),
            TOKEN1_FUNCTION_NAME,
            &tokens,
            from,
            Some(contract_address),
            None,
        )?);
    }

    Ok(async move {
        if let Some(dex) = dex {
            return Ok(dex);
        }

        let token0_address = token0_contract_addr
            .expect("token0_contract_addr should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap()
            .into_address()
            .unwrap();

        let token1_address = token1_contract_addr
            .expect("token1_contract_addr should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap()
            .into_address()
            .unwrap();

        let dex = DEX {
            address: pool_address.to_string(),
            token0_address: format!("{:?}", token0_address),
            token1_address: format!("{:?}", token1_address),
            ..DEX::default()
        };

        Ok(dex)
    })
}

fn get_dex_decimals_and_symbols<T: Transport + BatchTransport + 'static>(
    w3: Arc<Web3Instance<Batch<T>>>,
    mut dex: DEX,
    from: H160,
) -> Result<impl Future<Output = Result<DEX, CustomFeedError>>, CustomFeedError> {
    let tokens = vec![];

    let token0_contract_addr = H160::from_str(&dex.token0_address)
        .map_err(|_| CustomFeedError::AddressError(AddressError::InvalidAddress))?;
    let token1_contract_addr = H160::from_str(&dex.token1_address)
        .map_err(|_| CustomFeedError::AddressError(AddressError::InvalidAddress))?;

    let token0_contract = Arc::new(
        Contract::from_json(w3.eth(), token0_contract_addr, ERC20_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    let token1_contract = Arc::new(
        Contract::from_json(w3.eth(), token1_contract_addr, ERC20_ABI)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?,
    );

    let mut token0_decimals = None;
    let mut token1_decimals = None;
    let mut token0_symbol = None;
    let mut token1_symbol = None;

    if dex.token0_symbol.is_empty() || dex.token1_symbol.is_empty() {
        token0_decimals = Some(w3.get_call_result_promise(
            token0_contract.clone(),
            DECIMALS_FUNCTION_NAME,
            &tokens,
            from,
            Some(token0_contract_addr),
            None,
        )?);

        token1_decimals = Some(w3.get_call_result_promise(
            token1_contract.clone(),
            DECIMALS_FUNCTION_NAME,
            &tokens,
            from,
            Some(token1_contract_addr),
            None,
        )?);

        token0_symbol = Some(w3.get_call_result_promise(
            token0_contract.clone(),
            SYMBOL_FUNCTION_NAME,
            &tokens,
            from,
            Some(token0_contract_addr),
            None,
        )?);

        token1_symbol = Some(w3.get_call_result_promise(
            token1_contract.clone(),
            SYMBOL_FUNCTION_NAME,
            &tokens,
            from,
            Some(token1_contract_addr),
            None,
        )?);
    }

    Ok(async move {
        if !dex.token0_symbol.is_empty() && !dex.token1_symbol.is_empty() {
            return Ok(dex);
        }

        dex.token0_decimals = token0_decimals
            .expect("token0_decimals should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap()
            .into_uint()
            .unwrap()
            .as_u32();

        dex.token1_decimals = token1_decimals
            .expect("token1_decimals should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap()
            .into_uint()
            .unwrap()
            .as_u32();

        let token0_symbol = token0_symbol
            .expect("token0_symbol should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap();

        let token1_symbol = token1_symbol
            .expect("token1_symbol should not be None")
            .await
            .unwrap()?
            .pop()
            .unwrap();

        dex.token0_symbol = match token0_symbol {
            Token::String(s) => s,
            // Each byte must be greater than 0, because this token represents utf8 string in bytes32.
            // Thus, we can safely trim the trailing zeros and convert remaining bytes into a string.
            Token::FixedBytes(b) | Token::Bytes(b) => {
                let bytes = b.into_iter().take_while(|&c| c != 0).collect::<Vec<_>>();
                match String::from_utf8(bytes) {
                    Ok(s) => s,
                    _ => {
                        return Err(CustomFeedError::FailedToParseABI(
                            "Failed to parse token0 symbol".to_string(),
                        ))
                    }
                }
            }
            _ => {
                return Err(CustomFeedError::FailedToParseABI(
                    "Failed to parse token0 symbol".to_string(),
                ))
            }
        };

        dex.token1_symbol = match token1_symbol {
            Token::String(s) => s,
            // Each byte must be greater than 0, because this token represents utf8 string in bytes32.
            // Thus, we can safely trim the trailing zeros and convert remaining bytes into a string.
            Token::FixedBytes(b) | Token::Bytes(b) => {
                let bytes = b.into_iter().take_while(|&c| c != 0).collect::<Vec<_>>();
                match String::from_utf8(bytes) {
                    Ok(s) => s,
                    _ => {
                        return Err(CustomFeedError::FailedToParseABI(
                            "Failed to parse token1 symbol".to_string(),
                        ))
                    }
                }
            }
            _ => {
                return Err(CustomFeedError::FailedToParseABI(
                    "Failed to parse token1 symbol".to_string(),
                ))
            }
        };

        DEXList::add_dex(dex.address.clone(), dex.clone());

        Ok(dex)
    })
}

async fn get_dexes(
    chain_rpc: Vec<String>,
    pool_addresses: &[String],
    dex_type: DexType,
) -> Result<Vec<DEX>, CustomFeedError> {
    // w3 optimized for getting the token addresses in batch
    let w3 = Arc::new(web3::batch_instance(
        0, // TODO
        Some(chain_rpc.clone()),
        clone_with_state!(evm_rpc_canister),
        Some(get_token_address_batch_response_len(
            pool_addresses.len() as u64
        )),
    ));

    let mut dexes_with_token_pairs = Vec::with_capacity(pool_addresses.len());
    let from = address::to_h160(&canister::eth_address().await?.to_string())?;

    for pool_address in pool_addresses {
        dexes_with_token_pairs.push(get_dex_with_token_addresses_and_pool(
            w3.clone(),
            pool_address,
            dex_type.clone(),
            from,
        )?);
    }

    let balance_before = ic_cdk::api::canister_balance();
    w3.submit_batch().await?;

    let dexes = try_join_all(dexes_with_token_pairs).await?;
    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for getting dexes with token pairs: {}",
        balance_before - balance_after
    );

    // w3 optimized for getting decimals and symbols of dex in batch
    let w3 = Arc::new(web3::batch_instance(
        0, // TODO
        Some(chain_rpc.clone()),
        clone_with_state!(evm_rpc_canister),
        Some(get_decimals_and_symbols_batch_response_len(
            pool_addresses.len() as u64,
        )),
    ));
    let mut futures = Vec::with_capacity(dexes.len());
    for dex in dexes {
        futures.push(get_dex_decimals_and_symbols(w3.clone(), dex, from)?);
    }

    let balance_before = ic_cdk::api::canister_balance();
    w3.submit_batch().await?;

    let dexes = try_join_all(futures).await?;
    let balance_after = ic_cdk::api::canister_balance();
    log!(
        "cost for getting dexes with decimals and symbols: {}",
        balance_before - balance_after
    );

    Ok(dexes)
}
