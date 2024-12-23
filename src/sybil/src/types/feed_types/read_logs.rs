use candid::{CandidType, Nat};
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
    types::Log,
};
use serde::{Deserialize, Serialize};

use crate::{
    log,
    types::cache::{SignaturesCache, SignaturesCacheError},
    utils::{encoding::encode_packed, nat},
};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsMetadata {
    pub chain_id: u64,
    pub block_from: u64,
    pub block_to: u64,
    pub topics0: Vec<String>,
    pub topics1: Vec<String>,
    pub topics2: Vec<String>,
    pub topics3: Vec<String>,
    pub addresses: Vec<String>,
    pub timestamp: u64,
    #[serde(with = "super::big_num_serde")]
    pub fee: Nat,
    pub fee_symbol: String,
}

impl ReadLogsMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        let tokens = vec![
            Token::Uint(self.chain_id.into()),
            Token::Uint(self.block_from.into()),
            Token::Uint(self.block_to.into()),
            Token::Array(
                self.topics0
                    .clone()
                    .into_iter()
                    .map(Token::String)
                    .collect(),
            ),
            Token::Array(
                self.topics1
                    .clone()
                    .into_iter()
                    .map(Token::String)
                    .collect(),
            ),
            Token::Array(
                self.topics2
                    .clone()
                    .into_iter()
                    .map(Token::String)
                    .collect(),
            ),
            Token::Array(
                self.topics3
                    .clone()
                    .into_iter()
                    .map(Token::String)
                    .collect(),
            ),
            Token::Array(
                self.addresses
                    .clone()
                    .into_iter()
                    .map(Token::String)
                    .collect(),
            ),
            Token::Uint(self.timestamp.into()),
            Token::Uint(nat::to_u256(&self.fee)),
            Token::String(self.fee_symbol.clone()),
        ];

        tokens
    }

    pub fn encode(&self) -> Vec<u8> {
        let tuple = Token::Tuple(self.get_tokens());

        encode(&[tuple])
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsData {
    pub address: String,
    pub topics: Vec<String>,
    pub data: Vec<u8>,
    pub block_hash: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub transaction_index: u64,
    pub log_index: String,
    pub transaction_log_index: String,
    pub log_type: String,
    pub removed: bool,
}

impl From<Log> for ReadLogsData {
    fn from(log: Log) -> Self {
        let topics = log.topics.iter().map(|t| format!("{:?}", t)).collect();

        Self {
            address: format!("{:?}", log.address),
            topics,
            data: log.data.0,
            block_hash: log
                .block_hash
                .map(|h| format!("{:?}", h))
                .unwrap_or_default(),
            block_number: log.block_number.map(|n| n.as_u64()).unwrap_or_default(),
            transaction_hash: log
                .transaction_hash
                .map(|h| format!("{:?}", h))
                .unwrap_or_default(),
            transaction_index: log
                .transaction_index
                .map(|i| i.as_u64())
                .unwrap_or_default(),
            log_index: log
                .log_index
                .map(|i| format!("{:?}", i))
                .unwrap_or_default(),
            transaction_log_index: log
                .transaction_log_index
                .map(|i| format!("{:?}", i))
                .unwrap_or_default(),
            log_type: log.log_type.clone().unwrap_or_default(),
            removed: log.removed.unwrap_or_default(),
        }
    }
}

impl ReadLogsData {
    fn get_tokens(&self) -> Vec<Token> {
        let tokens = vec![
            Token::String(self.address.clone()),
            Token::Array(self.topics.clone().into_iter().map(Token::String).collect()),
            Token::Bytes(self.data.clone()),
            Token::String(self.block_hash.clone()),
            Token::Uint(self.block_number.into()),
            Token::String(self.transaction_hash.clone()),
            Token::Uint(self.transaction_index.into()),
            Token::String(self.log_index.clone()),
            Token::String(self.transaction_log_index.clone()),
            Token::String(self.log_type.clone()),
            Token::Bool(self.removed),
        ];

        tokens
    }

    pub fn get_token(&self) -> Token {
        Token::Tuple(self.get_tokens())
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsResult {
    pub data: Vec<ReadLogsData>,
    pub meta: Option<ReadLogsMetadata>,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl ReadLogsResult {
    fn encode_packed(&self) -> Vec<u8> {
        let data_tokens: Vec<_> = self.data.iter().map(|d| d.get_token()).collect();

        let mut data_to_encode = Vec::new();

        data_to_encode.push(Token::Bytes(encode(&data_tokens)));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        encode_packed(&data_to_encode).expect("tokens should be valid")
    }

    pub fn encode(&self) -> Vec<u8> {
        let data_tokens: Vec<_> = self.data.iter().map(|d| d.get_token()).collect();

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
