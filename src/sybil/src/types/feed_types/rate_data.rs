use candid::CandidType;
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    log,
    types::cache::{SignaturesCache, SignaturesCacheError},
    utils::encoding::encode_packed,
};

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

impl AssetData {
    pub fn get_tokens(self) -> Vec<Token> {
        match self {
            AssetData::DefaultPriceFeed {
                symbol,
                rate,
                decimals,
                timestamp,
            } => vec![
                Token::String(symbol),
                Token::Uint(rate.into()),
                Token::Uint(decimals.into()),
                Token::Uint(timestamp.into()),
            ],
            AssetData::CustomPriceFeed {
                symbol,
                rate,
                decimals,
                timestamp,
            } => vec![
                Token::String(symbol),
                Token::Uint(rate.into()),
                Token::Uint(decimals.into()),
                Token::Uint(timestamp.into()),
            ],
            AssetData::CustomNumber {
                id,
                value,
                decimals,
            } => vec![
                Token::String(id),
                Token::Uint(value.into()),
                Token::Uint(decimals.into()),
            ],
            AssetData::CustomString { id, value } => {
                vec![Token::String(id), Token::String(value)]
            }
        }
    }

    pub fn get_token(self) -> Token {
        Token::Tuple(self.get_tokens())
    }

    pub fn encode(&self) -> Vec<u8> {
        encode(&self.clone().get_tokens())
    }
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
        let encoded_packed =
            encode_packed(&self.data.clone().get_tokens()).expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut tokens = self.data.clone().get_tokens();

        tokens.push(if let Some(signature) = &self.signature {
            Token::Bytes(hex::decode(signature.clone()).unwrap())
        } else {
            Token::Bytes(vec![])
        });

        encode(&tokens)
    }

    pub async fn sign(&mut self) -> Result<(), RateDataError> {
        let sign_data = self.encode_packed();

        log!(
            "asset data signed: 0x{}",
            hex::encode(keccak256(&sign_data))
        );

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));

        Ok(())
    }
}

#[derive(Clone, Default, Debug, CandidType, Serialize, Deserialize)]
pub struct MultipleAssetsDataResult {
    pub data: Vec<AssetData>,
    pub signature: Option<String>,
}

impl MultipleAssetsDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let tokens = self
            .data
            .iter()
            .map(|d| d.clone().get_tokens())
            .flatten()
            .collect::<Vec<Token>>();

        encode_packed(&tokens).expect("tokens should be valid")
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut tokens = self
            .data
            .iter()
            .map(|d| d.clone().get_tokens())
            .flatten()
            .collect::<Vec<Token>>();

        tokens.push(if let Some(signature) = &self.signature {
            Token::String(signature.clone())
        } else {
            Token::String("".to_string())
        });

        encode(&tokens)
    }

    pub async fn sign(&mut self) -> Result<(), RateDataError> {
        let sign_data = self.encode_packed();

        log!(
            "multiple assets data signed: 0x{}",
            hex::encode(keccak256(&sign_data))
        );

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));

        Ok(())
    }
}
