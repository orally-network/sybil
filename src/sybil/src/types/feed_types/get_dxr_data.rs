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

        encode(&vec![tuple])
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
        encode(&vec![tuple])
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

        let encoded_packed = encode_packed(&data_to_encode).expect("tokens should be valid");

        encoded_packed
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
