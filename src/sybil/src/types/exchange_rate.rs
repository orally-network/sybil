use candid::{CandidType, Principal};
use serde::Deserialize;
use thiserror::Error;

use ic_cdk::api::call::{call_with_payment, CallResult};

use crate::log;

pub const CYCLES_TO_SEND: u64 = 10_000_000_000;

#[derive(CandidType, Deserialize, Debug, Clone, Default)]
pub enum AssetClass {
    Cryptocurrency,
    #[default]
    FiatCurrency,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default)]
pub struct Asset {
    pub(crate) class: AssetClass,
    pub(crate) symbol: String,
}

#[derive(CandidType, Deserialize, Debug, Clone)]
pub struct GetExchangeRateRequest {
    pub(crate) timestamp: Option<u64>,
    pub(crate) quote_asset: Asset,
    pub(crate) base_asset: Asset,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default)]
pub struct ExchangeRateMetadata {
    pub decimals: u32,
    pub forex_timestamp: Option<u64>,
    pub quote_asset_num_received_rates: u64,
    pub base_asset_num_received_rates: u64,
    pub base_asset_num_queried_sources: u64,
    pub standard_deviation: u64,
    pub quote_asset_num_queried_sources: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default)]
pub struct ExchangeRate {
    pub metadata: ExchangeRateMetadata,
    pub(crate) rate: u64,
    pub timestamp: u64,
    quote_asset: Asset,
    base_asset: Asset,
}

#[derive(Error, CandidType, Deserialize, Debug, Clone, PartialEq)]
pub enum ExchangeRateError {
    #[error("Anonymous principal not allowed")]
    AnonymousPrincipalNotAllowed,
    #[error("Pending")]
    Pending,
    #[error("Crypto base asset not found")]
    CryptoBaseAssetNotFound,
    #[error("Crypto Quote Asset Not Found")]
    CryptoQuoteAssetNotFound,
    #[error("Stablecoin rate not found")]
    StablecoinRateNotFound,
    #[error("Stablecoin rate too few rates")]
    StablecoinRateTooFewRates,
    #[error("Stablecoin rate zero rate")]
    StablecoinRateZeroRate,
    #[error("Forex invalid timestamp")]
    ForexInvalidTimestamp,
    #[error("Forex base asset not found")]
    ForexBaseAssetNotFound,
    #[error("Forex quote asset not found")]
    ForexQuoteAssetNotFound,
    #[error("Forex assets not found")]
    ForexAssetsNotFound,
    #[error("Rate limited")]
    RateLimited,
    #[error("Not enough cycles")]
    NotEnoughCycles,
    #[error("Failed to accept cycles")]
    FailedToAcceptCycles,
    #[error("Inconsistent rates received")]
    InconsistentRatesReceived,
    #[error("Unexpected error: {description:?}")]
    Other { code: u32, description: String },
}

#[derive(CandidType, Deserialize, Debug)]
pub enum GetExchangeRateResult {
    Ok(ExchangeRate),
    Err(ExchangeRateError),
}

impl From<GetExchangeRateResult> for Result<ExchangeRate, ExchangeRateError> {
    fn from(result: GetExchangeRateResult) -> Self {
        match result {
            GetExchangeRateResult::Ok(rate) => Ok(rate),
            GetExchangeRateResult::Err(err) => Err(err),
        }
    }
}

pub struct Service(pub Principal);

impl Service {
    pub async fn get_exchange_rate(
        &self,
        arg0: GetExchangeRateRequest,
    ) -> CallResult<(GetExchangeRateResult,)> {
        log!(
            "Sending get_exchange_rate request to xrc ({}) with args: {:?} and cycles: {}",
            self.0,
            arg0,
            CYCLES_TO_SEND
        );

        let result = call_with_payment(self.0, "get_exchange_rate", (arg0,), CYCLES_TO_SEND).await;

        log!("got response from xrc: {:#?}", result);

        result
    }
}
