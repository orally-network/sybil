use std::str::FromStr;

use candid::{CandidType, Nat};
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
    types::{H160, U256},
};
use serde::{Deserialize, Serialize};

use crate::{
    log,
    types::cache::{SignaturesCache, SignaturesCacheError},
    utils::{encoding::encode_packed, nat},
};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadContractMetadata {
    pub chain_id: u64,
    pub contract_address: String,
    pub method: String,
    pub params: String,
    pub block_number: u64,
    pub timestamp: u64,
    #[serde(with = "super::big_num_serde")]
    pub fee: Nat,
    pub fee_symbol: String,
}

impl ReadContractMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        vec![
            Token::Uint(self.chain_id.into()),
            Token::Address(self.contract_address.parse().unwrap()),
            Token::String(self.method.clone()),
            Token::String(self.params.clone()),
            Token::Uint(self.block_number.into()),
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

/// SolidityToken is a representation of a web3_rs Token, but with CandidType support
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum SolidityToken {
    Address(String),
    FixedBytes(ic_web3_rs::ethabi::FixedBytes),
    Bytes(ic_web3_rs::ethabi::Bytes),
    Int(String),
    Uint(String),
    Bool(bool),
    String(String),
    FixedArray(Vec<SolidityToken>),
    Array(Vec<SolidityToken>),
    Tuple(Vec<SolidityToken>),
}

impl From<Token> for SolidityToken {
    fn from(token: Token) -> Self {
        match token {
            Token::Address(address) => SolidityToken::Address(format!("{:?}", address)),
            Token::FixedBytes(bytes) => SolidityToken::FixedBytes(bytes),
            Token::Bytes(bytes) => SolidityToken::Bytes(bytes),
            Token::Int(int) => SolidityToken::Int(format!("{}", int)),
            Token::Uint(uint) => SolidityToken::Uint(format!("{}", uint)),
            Token::Bool(boolean) => SolidityToken::Bool(boolean),
            Token::String(string) => SolidityToken::String(string),
            Token::FixedArray(tokens) => {
                SolidityToken::FixedArray(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Array(tokens) => {
                SolidityToken::Array(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Tuple(tokens) => {
                SolidityToken::Tuple(tokens.into_iter().map(SolidityToken::from).collect())
            }
        }
    }
}

impl From<SolidityToken> for Token {
    fn from(token: SolidityToken) -> Self {
        match token {
            SolidityToken::Address(address) => Token::Address(H160::from_str(&address).unwrap()),
            SolidityToken::FixedBytes(bytes) => Token::FixedBytes(bytes),
            SolidityToken::Bytes(bytes) => Token::Bytes(bytes),
            SolidityToken::Int(int) => Token::Int(U256::from_str_radix(&int, 10).unwrap()),
            SolidityToken::Uint(uint) => Token::Uint(U256::from_str_radix(&uint, 10).unwrap()),
            SolidityToken::Bool(boolean) => Token::Bool(boolean),
            SolidityToken::String(string) => Token::String(string),
            SolidityToken::FixedArray(tokens) => {
                Token::FixedArray(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Array(tokens) => {
                Token::Array(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Tuple(tokens) => {
                Token::Tuple(tokens.into_iter().map(Token::from).collect())
            }
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadContractResult {
    pub data: Vec<SolidityToken>,
    pub meta: Option<ReadContractMetadata>,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl ReadContractResult {
    fn encode_packed(&self) -> Vec<u8> {
        let data_tokens: Vec<_> = self.data.clone().into_iter().map(Token::from).collect();

        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(encode(&data_tokens)));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        let encoded_packed = encode_packed(&data_to_encode).expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let data_tokens: Vec<_> = self.data.clone().into_iter().map(Token::from).collect();

        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(encode(&data_tokens)));

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
