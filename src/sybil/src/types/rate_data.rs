use candid::CandidType;
use ic_web3_rs::ethabi::Token;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::utils::encoding::encode_packed;

use super::cache::{SignaturesCache, SignaturesCacheError};

#[derive(Error, Debug)]
pub enum RateDataError {
    #[error("Singatures cache error: {0}")]
    SignaturesCacheError(#[from] SignaturesCacheError),
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum AssetData {
    DefaultPriceFeed {
        symbol: String,
        rate: u64,
        decimals: u64,
        timestamp: u64,
    },
    CustomPriceFeed {
        symbol: String,
        rate: u64,
        decimals: u64,
        timestamp: u64,
    },
    CustomNumber {
        id: String,
        value: u64,
        decimals: u64,
    },
    CustomString {
        id: String,
        value: String,
    },
}

impl Default for AssetData {
    fn default() -> Self {
        AssetData::DefaultPriceFeed {
            symbol: "".to_string(),
            rate: 0,
            decimals: 0,
            timestamp: 0,
        }
    }
}

#[derive(Clone, Default, Debug, CandidType, Serialize, Deserialize)]
pub struct AssetDataResult {
    pub data: AssetData,
    pub signature: Option<String>,
}

impl AssetDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = match self.data.clone() {
            AssetData::DefaultPriceFeed {
                symbol,
                rate,
                decimals,
                timestamp,
            } => vec![
                Token::String(symbol.clone()),
                Token::Uint(rate.into()),
                Token::Uint(decimals.into()),
                Token::Uint(timestamp.into()),
            ],
            AssetData::CustomPriceFeed {
                symbol,
                rate,
                decimals,
                timestamp,
            } => {
                vec![
                    Token::String(symbol.clone()),
                    Token::Uint(rate.into()),
                    Token::Uint(decimals.into()),
                    Token::Uint(timestamp.into()),
                ]
            }
            AssetData::CustomNumber {
                id,
                value,
                decimals,
            } => {
                vec![
                    Token::String(id.clone()),
                    Token::Uint(value.into()),
                    Token::Uint(decimals.into()),
                ]
            }
            AssetData::CustomString { id, value } => {
                vec![Token::String(id.clone()), Token::String(value.clone())]
            }
        };

        encode_packed(&raw_data).expect("tokens should be valid")
    }

    pub async fn sign(&mut self) -> Result<(), RateDataError> {
        let sign_data = self.encode_packed();

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));

        Ok(())
    }
}
