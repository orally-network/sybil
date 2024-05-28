use std::cmp::{max, min};

use candid::Nat;
use ic_web3_rs::types::U256;

use crate::{
    stringify_func_call,
    types::{
        cache::Cache,
        feed_types::rate_data::AssetData,
        feeds::{Feed, FeedError, FeedStorage, DEFAULT_UPDATE_FREQ},
    },
};

use super::nat;

// COEFFICIENT for the conversion fee, in percentage
const CONVERTION_FEE_PERCENT: u64 = 50;
const CACHE_TTL_SEC_FOR_CONVERTION: u64 = 60 * 60 * 3; // 3 hours

#[inline(always)]
pub fn u64_to_i64(num: u64) -> i64 {
    if num < i64::MAX as u64 {
        num as i64
    } else {
        i64::MAX
    }
}

// Converts USD (nat with 6 decimals) to ETH (Nat with 18 decimals) with default rate and additional fee for the conversion
pub async fn convert_usd_to_eth(value_usd: Nat, value_decimals: u64) -> Result<Nat, FeedError> {
    let rate = Feed {
        id: "ETH/USD".to_string(),
        update_freq: DEFAULT_UPDATE_FREQ,
        ..Default::default()
    };

    //Using _get_xrc_data is just slightly beter because it will use the same cache as the regular get_xrc_data
    let func_signature = stringify_func_call!(_get_xrc_data(rate.id, false));

    let mut cache_builder = Cache::with(func_signature, FeedStorage::get_default_rate(&rate, None));

    cache_builder.with_cache_ttl(CACHE_TTL_SEC_FOR_CONVERTION);

    let (_, rate) = cache_builder.evaluate().await?;

    let AssetData::DefaultPriceFeed { rate, decimals, .. } = rate.data else {
        unreachable!("xrc data should be default price feed");
    };

    let value_usd = value_usd.clone() + (value_usd * CONVERTION_FEE_PERCENT / 100);

    Ok(convert_to_eth_weis(
        value_usd,
        value_decimals,
        rate,
        decimals,
    ))
}

/// Converts sybils USD (nat with 6 decimals) to ETH (Nat with 18 decimals) with provided rate
#[inline(always)]
pub fn convert_to_eth_weis(
    value_usd: Nat,
    decimals_usd: u64,
    rate: u64,
    decimals_rate: u64,
) -> Nat {
    let value_usd: U256 = nat::to_u256(&value_usd);
    let rate: U256 = rate.into();

    let eth_decimals = 18.into();
    let decimals_diff =
        U256::from(max(decimals_usd, decimals_rate) - min(decimals_usd, decimals_rate));

    let a = value_usd * U256::from(10).pow(eth_decimals) * U256::from(10).pow(decimals_diff);
    let b = rate;

    let res = a / b;
    nat::from_u256(&res)
}
