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

use super::rate_data::AssetData;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetAssetDataMetadata {
    pub id: String,
    pub timestamp: u64,
    #[serde(with = "super::big_num_serde")]
    pub fee: Nat,
    pub fee_symbol: String,
}

impl GetAssetDataMetadata {
    fn get_tokens(&self) -> Vec<Token> {
        vec![
            Token::String(self.id.clone()),
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
pub struct GetAssetDataResult {
    pub data: AssetData,
    pub meta: Option<GetAssetDataMetadata>,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl GetAssetDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        let data_tokens = self.data.clone().get_tokens();
        data_to_encode.push(Token::Bytes(encode(&vec![Token::Tuple(data_tokens)])));

        if let Some(meta) = &self.meta {
            data_to_encode.push(Token::Bytes(meta.encode()));
        }

        let encoded_packed = encode_packed(&data_to_encode).expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut data_to_encode = Vec::new();

        let data_tokens = self.data.clone().get_tokens();
        data_to_encode.push(Token::Bytes(encode(&vec![Token::Tuple(data_tokens)])));

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
