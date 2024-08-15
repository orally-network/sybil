use ic_cdk::update;
use sybil_utils::cycles_count;

use crate::{
    log,
    methods::balances,
    stringify_func_call,
    types::{
        balances::Balances,
        cache::{self, Cache},
        feed_types::{
            get_xrc_data::{GetXRCData, GetXRCDataMetadata, GetXRCDataResult},
            rate_data::AssetData,
        },
        feeds::{Feed, FeedError, FeedStorage, DEFAULT_UPDATE_FREQ},
    },
    utils::{canister, convertion::convert_usd_to_eth, siwe, time::in_seconds},
};

#[update]
pub async fn get_xrc_data(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<GetXRCDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let func_signature = stringify_func_call!(_get_xrc_data(id, false));
    let mut cache_builder = Cache::with(func_signature, _get_xrc_data(id, false, Some(payer)));

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[update]
pub async fn get_xrc_data_with_proof(
    id: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<GetXRCDataResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    let func_signature = stringify_func_call!(_get_xrc_data(id, true));
    let cache_builder = Cache::with(func_signature, _get_xrc_data(id, true, Some(payer)));

    cache_builder
        .evaluate()
        .await
        .map_err(|e| format!("failed to get asset data: {}", e))
}

#[inline]
#[cycles_count]
pub async fn _get_xrc_data(
    id: String,
    with_signature: bool,
    payer: Option<String>,
) -> Result<GetXRCDataResult, FeedError> {
    let rate = Feed {
        id: id.clone(),
        update_freq: DEFAULT_UPDATE_FREQ,
        ..Default::default()
    };

    let (cost, rate) = FeedStorage::get_default_rate(&rate, None).await?;

    let rate = rate.data;

    let AssetData::DefaultPriceFeed {
        symbol,
        rate,
        decimals,
        timestamp,
    } = rate
    else {
        unreachable!("xrc data should be default price feed");
    };

    let mut result = GetXRCDataResult {
        data: GetXRCData {
            symbol,
            rate,
            decimals,
            timestamp,
        },
        meta: GetXRCDataMetadata {
            id: id.clone(),
            timestamp: in_seconds(),
            fee: 0.into(),
            fee_symbol: "ETH".to_string(), // TODO: it's hardcoded, change it properly
        },
        signature: None,
        bytes: None,
    };

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
