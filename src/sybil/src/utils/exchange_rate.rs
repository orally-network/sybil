use thiserror::Error;

use ic_cdk::api::call::{call_with_payment, CallResult};
use ic_cdk::export::{
    candid::{CandidType, Deserialize},
    Principal,
};

pub const CYCLES_TO_SEND: u64 = 10_000_000_000;

#[derive(CandidType, Deserialize, Debug)]
pub enum AssetClass {
    Cryptocurrency,
    FiatCurrency,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct Asset {
    pub(crate) class: AssetClass,
    pub(crate) symbol: String,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct GetExchangeRateRequest {
    pub(crate) timestamp: Option<u64>,
    pub(crate) quote_asset: Asset,
    pub(crate) base_asset: Asset,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ExchangeRateMetadata {
    pub decimals: u32,
    forex_timestamp: Option<u64>,
    quote_asset_num_received_rates: u64,
    pub base_asset_num_received_rates: u64,
    pub base_asset_num_queried_sources: u64,
    standard_deviation: u64,
    quote_asset_num_queried_sources: u64,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ExchangeRate {
    pub metadata: ExchangeRateMetadata,
    pub(crate) rate: u64,
    pub timestamp: u64,
    quote_asset: Asset,
    base_asset: Asset,
}

#[derive(Error, CandidType, Deserialize, Debug)]
pub enum ExchangeRateError {
    #[error("Anonymous principal not allowed")]
    AnonymousPrincipalNotAllowed,
    #[error("Crypto Quote Asset Not Found")]
    CryptoQuoteAssetNotFound,
    #[error("Failed to accept cycles")]
    FailedToAcceptCycles,
    #[error("Forex base asset not found")]
    ForexBaseAssetNotFound,
    #[error("Crypto base asset not found")]
    CryptoBaseAssetNotFound,
    #[error("Stablecoin rate too few rates")]
    StablecoinRateTooFewRates,
    #[error("Forex assets not found")]
    ForexAssetsNotFound,
    #[error("Inconsistent rates received")]
    InconsistentRatesReceived,
    #[error("Rate limited")]
    RateLimited,
    #[error("Stablecoin rate zero rate")]
    StablecoinRateZeroRate,
    #[error("Unexpected error: {description:?}")]
    Other { code: u32, description: String },
    #[error("Forex invalid timestamp")]
    ForexInvalidTimestamp,
    #[error("Not enough cycles")]
    NotEnoughCycles,
    #[error("Forex quote asset not found")]
    ForexQuoteAssetNotFound,
    #[error("Stablecoin rate not found")]
    StablecoinRateNotFound,
    #[error("pending")]
    Pending,
}

#[derive(CandidType, Deserialize, Debug)]
pub enum GetExchangeRateResult {
    Ok(ExchangeRate),
    Err(ExchangeRateError),
}

pub struct Service(pub Principal);

impl Service {
    pub async fn get_exchange_rate(
        &self,
        arg0: GetExchangeRateRequest,
    ) -> CallResult<(GetExchangeRateResult,)> {
        call_with_payment(self.0, "get_exchange_rate", (arg0,), CYCLES_TO_SEND).await
    }
}
