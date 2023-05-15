use std::str::FromStr;

use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use ic_web3::{
    ethabi::Token,
    ic::recover_address,
    signing::{hash_message, keccak256},
    types::H160,
};

use anyhow::{Context, Result};

use crate::utils::encoding::encode_packed;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub timestamp: u64,
    pub decimals: u32,
}

impl RateDataLight {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = vec![
            Token::String(self.symbol.clone()),
            Token::Uint(self.rate.into()),
            Token::Uint(self.timestamp.into()),
            Token::Uint(self.decimals.into()),
        ];

        encode_packed(&raw_data).expect("Tokens is always valid")
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CustomPairData {
    pub data: RateDataLight,
    pub signature: String,
}

impl CustomPairData {
    pub fn verify(&self, pub_key: &H160) -> Result<()> {
        let sign_data = hash_message(keccak256(&self.data.encode_packed()));

        let signature = hex::decode(&self.signature)?;

        let (signature, rec_id) = signature.split_at(64);

        let rec_id = *rec_id.first().context("Invalid signature")?;

        let signer = recover_address(sign_data.as_bytes().to_vec(), signature.to_vec(), rec_id);

        if *pub_key != H160::from_str(&signer)? {
            return Err(anyhow::anyhow!("Invalid signature"));
        }

        Ok(())
    }
}
