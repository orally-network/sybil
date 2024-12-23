use std::fmt::{Display, Formatter};

use candid::{CandidType, Nat};
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
};
use serde::{Deserialize, Serialize};

use crate::{
    log,
    types::cache::{SignaturesCache, SignaturesCacheError},
    utils::{encoding::encode_packed, nat},
};

#[derive(CandidType, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Aggregation {
    // Get the average price from the specified range
    #[serde(rename = "avg_from_range")]
    AvgFromRange((u64, u64)),
    // Get the average price from the last N blocks
    #[serde(rename = "avg_from_last_blocks")]
    AvgFromLastBlocks(u64),
}

#[derive(CandidType, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
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

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetDXRDataMetadata {
    // pub chain_id: u64,
    // pub pool_address: String,
    // pub block_numbers: Vec<u64>,
    // pub dex_type: String,
    // pub reverse_pair: bool,
    pub timestamp: u64,
    #[serde(with = "super::big_num_serde")]
    pub fee: Nat,
    pub fee_symbol: String,
}

impl GetDXRDataMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        vec![
            // Token::Uint(self.chain_id.into()),
            // Token::String(self.pool_address.clone()),
            // Token::Array(
            //     self.block_numbers
            //         .iter()
            //         .map(|block_number| Token::Uint((*block_number).into()))
            //         .collect(),
            // ),
            // Token::String(self.dex_type.clone()),
            // Token::Bool(self.reverse_pair),
            Token::Uint(self.timestamp.into()),
            Token::Uint(nat::to_u256(&self.fee)),
            Token::String(self.fee_symbol.clone()),
        ]
    }

    pub fn encode(&self) -> Vec<u8> {
        let tuple = Token::Tuple(self.get_tokens());

        encode(&[tuple])
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetDXRData {
    pub feed_id: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
}

impl GetDXRData {
    pub fn get_tokens(&self) -> Vec<Token> {
        vec![
            Token::String(self.feed_id.clone()),
            Token::Uint(self.rate.into()),
            Token::Uint(self.decimals.into()),
            Token::Uint(self.timestamp.into()),
        ]
    }

    pub fn encode(&self) -> Vec<u8> {
        let tuple = Token::Tuple(self.get_tokens());
        encode(&[tuple])
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetDXRDataResult {
    pub data: GetDXRData,
    pub meta: Option<GetDXRDataMetadata>,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl GetDXRDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(self.data.encode()));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        encode_packed(&data_to_encode).expect("tokens should be valid")
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(self.data.encode()));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        if let Some(signature) = &self.signature {
            data_to_encode.push(Token::Bytes(hex::decode(signature.clone()).unwrap()));
        };

        encode(&data_to_encode)
    }

    pub async fn sign(&mut self) -> Result<(), SignaturesCacheError> {
        let balance_before = ic_cdk::api::canister_balance();
        let sign_data = self.encode_packed();

        log!(
            "asset data signed: 0x{}",
            hex::encode(keccak256(&sign_data))
        );

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));
        let balance_after = ic_cdk::api::canister_balance();
        log!("Cost for sign: {}", balance_before - balance_after);

        Ok(())
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetDXRDataBatchResult {
    pub data: Vec<GetDXRData>,
    pub meta: Option<GetDXRDataMetadata>,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl GetDXRDataBatchResult {
    fn encode_data(&self) -> Vec<u8> {
        encode(&[Token::Array(
            self.data
                .iter()
                .map(|data| Token::Bytes(data.encode()))
                .collect(),
        )])
    }

    fn encode_packed(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(self.encode_data()));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        encode_packed(&data_to_encode).expect("tokens should be valid")
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(self.encode_data()));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        if let Some(signature) = &self.signature {
            data_to_encode.push(Token::Bytes(hex::decode(signature.clone()).unwrap()));
        };

        encode(&data_to_encode)
    }

    pub async fn sign(&mut self) -> Result<(), SignaturesCacheError> {
        let balance_before = ic_cdk::api::canister_balance();
        let sign_data = self.encode_packed();

        log!(
            "asset data signed: 0x{}",
            hex::encode(keccak256(&sign_data))
        );

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));
        let balance_after = ic_cdk::api::canister_balance();
        log!("Cost for sign: {}", balance_before - balance_after);

        Ok(())
    }
}
