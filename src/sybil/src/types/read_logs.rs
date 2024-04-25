use candid::CandidType;
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
    types::Log,
};
use serde::{Deserialize, Serialize};

use crate::{log, types::cache::SignaturesCache, utils::encoding::encode_packed};

use super::cache::SignaturesCacheError;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsMetadata {
    pub chain_id: u64,
    pub block_from: Option<u64>,
    pub block_to: Option<u64>,
    pub topics0: Option<Vec<String>>,
    pub topics1: Option<Vec<String>>,
    pub topics2: Option<Vec<String>>,
    pub topics3: Option<Vec<String>>,
    pub addresses: Option<Vec<String>>,
    pub timestamp: u64,
}

impl ReadLogsMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        let mut tokens = vec![Token::Uint(self.chain_id.into())];

        if let Some(block_from) = self.block_from {
            tokens.push(Token::Uint(block_from.into()));
        }

        if let Some(block_to) = self.block_to {
            tokens.push(Token::Uint(block_to.into()));
        }

        if let Some(topics0) = &self.topics0 {
            tokens.push(Token::Array(
                topics0
                    .into_iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ));
        }

        if let Some(topics1) = &self.topics1 {
            tokens.push(Token::Array(
                topics1
                    .into_iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ));
        }

        if let Some(topics2) = &self.topics2 {
            tokens.push(Token::Array(
                topics2
                    .into_iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ));
        }

        if let Some(topics3) = &self.topics3 {
            tokens.push(Token::Array(
                topics3
                    .into_iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ));
        }

        if let Some(addresses) = &self.addresses {
            tokens.push(Token::Array(
                addresses
                    .into_iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ));
        }

        tokens.push(Token::Uint(self.timestamp.into()));

        tokens
    }

    pub fn encode(&self) -> Vec<u8> {
        let tuple = Token::Tuple(self.get_tokens());

        encode(&vec![tuple])
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsData {
    pub address: String,
    pub topics: Vec<String>,
    pub data: Vec<u8>,
    pub block_hash: Option<String>,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub transaction_index: Option<u64>,
    pub log_index: Option<String>,
    pub transaction_log_index: Option<String>,
    pub log_type: Option<String>,
    pub removed: Option<bool>,
}

impl From<Log> for ReadLogsData {
    fn from(log: Log) -> Self {
        let topics = log.topics.iter().map(|t| format!("{:?}", t)).collect();

        Self {
            address: format!("{:?}", log.address),
            topics,
            data: log.data.0,
            block_hash: log.block_hash.map(|h| format!("{:?}", h)),
            block_number: log.block_number.map(|n| n.as_u64()),
            transaction_hash: log.transaction_hash.map(|h| format!("{:?}", h)),
            transaction_index: log.transaction_index.map(|i| i.as_u64()),
            log_index: log.log_index.map(|i| format!("{:?}", i)),
            transaction_log_index: log.transaction_log_index.map(|i| format!("{:?}", i)),
            log_type: log.log_type.clone(),
            removed: log.removed,
        }
    }
}

impl ReadLogsData {
    fn get_tokens(&self) -> Vec<Token> {
        let mut tokens = vec![
            Token::String(self.address.clone()),
            Token::Array(self.topics.clone().into_iter().map(Token::String).collect()),
            Token::Bytes(self.data.clone()),
        ];

        if let Some(block_hash) = &self.block_hash {
            tokens.push(Token::String(block_hash.clone()));
        }

        if let Some(block_number) = self.block_number {
            tokens.push(Token::Uint(block_number.into()));
        }

        if let Some(transaction_hash) = &self.transaction_hash {
            tokens.push(Token::String(transaction_hash.clone()));
        }

        if let Some(transaction_index) = self.transaction_index {
            tokens.push(Token::Uint(transaction_index.into()));
        }

        if let Some(log_index) = &self.log_index {
            tokens.push(Token::String(log_index.clone()));
        }

        if let Some(transaction_log_index) = &self.transaction_log_index {
            tokens.push(Token::String(transaction_log_index.clone()));
        }

        if let Some(log_type) = &self.log_type {
            tokens.push(Token::String(log_type.clone()));
        }

        if let Some(removed) = self.removed {
            tokens.push(Token::Bool(removed));
        }

        tokens
    }

    pub fn get_token(&self) -> Token {
        Token::Tuple(self.get_tokens())
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct ReadLogsResult {
    pub data: Vec<ReadLogsData>,
    pub meta: ReadLogsMetadata,
    pub signature: Option<String>,
}

impl ReadLogsResult {
    fn encode_packed(&self) -> Vec<u8> {
        let tokens = self
            .data
            .iter()
            .map(|log| log.get_token())
            .collect::<Vec<Token>>();

        let encode_data = encode(&tokens);

        let encode_meta = self.meta.encode();

        let encoded_packed =
            encode_packed(&vec![Token::Bytes(encode_data), Token::Bytes(encode_meta)])
                .expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let tokens = self
            .data
            .iter()
            .map(|log| log.get_token())
            .collect::<Vec<Token>>();

        let encode_data = encode(&tokens);

        let encode_meta = self.meta.encode();

        let encode_signature = if let Some(signature) = &self.signature {
            hex::decode(signature.clone()).unwrap()
        } else {
            vec![]
        };

        encode(&vec![
            Token::Bytes(encode_data),
            Token::Bytes(encode_meta),
            Token::Bytes(encode_signature),
        ])
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
