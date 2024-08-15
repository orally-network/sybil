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

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct GetXRCDataMetadata {
    pub id: String,
    pub timestamp: u64,
    #[serde(with = "super::big_num_serde")]
    pub fee: Nat,
    pub fee_symbol: String,
}

impl GetXRCDataMetadata {
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
pub struct GetXRCData {
    pub symbol: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
}

impl GetXRCData {
    pub fn get_tokens(&self) -> Vec<Token> {
        vec![
            Token::String(self.symbol.clone()),
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
pub struct GetXRCDataResult {
    pub data: GetXRCData,
    pub meta: GetXRCDataMetadata,
    pub signature: Option<String>,
    pub bytes: Option<String>,
}

impl GetXRCDataResult {
    fn encode_packed(&self) -> Vec<u8> {
        let encode_data = self.data.encode();
        let encode_meta = self.meta.encode();

        let encoded_packed =
            encode_packed(&vec![Token::Bytes(encode_data), Token::Bytes(encode_meta)])
                .expect("tokens should be valid");

        encoded_packed
    }

    pub fn encode(&self) -> Vec<u8> {
        let encode_data = self.data.encode();
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
