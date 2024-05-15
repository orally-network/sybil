use candid::CandidType;
use ic_web3_rs::{
    ethabi::{encode, Token},
    signing::keccak256,
};
use serde::{Deserialize, Serialize};

use crate::{
    log,
    types::cache::{SignaturesCache, SignaturesCacheError},
    utils::encoding::encode_packed,
};

use super::rate_data::AssetData;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetAssetDataMetadata {
    pub id: String,
    pub timestamp: u64,
}

impl GetAssetDataMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        vec![
            Token::String(self.id.clone()),
            Token::Uint(self.timestamp.into()),
        ]
    }

    pub fn encode(&self) -> Vec<u8> {
        let tuple = Token::Tuple(self.get_tokens());

        encode(&vec![tuple])
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetAssetDataResult {
    pub data: AssetData,
    pub meta: GetAssetDataMetadata,
    pub signature: Option<String>,
}

impl GetAssetDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let data_tokens = self.data.clone().get_tokens();

        let encode_data = encode(&vec![Token::Tuple(data_tokens)]);

        let encode_meta = self.meta.encode();

        let encoded_packed =
            encode_packed(&vec![Token::Bytes(encode_data), Token::Bytes(encode_meta)])
                .expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let data_tokens = self.data.clone().get_tokens();

        let encode_data = encode(&vec![Token::Tuple(data_tokens)]);

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
