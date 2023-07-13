use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use ic_web3_rs::ethabi::Token;
use thiserror::Error;

use crate::utils::encoding::encode_packed;

use super::cache::{SignaturesCache, SignaturesCacheError};

#[derive(Error, Debug)]
pub enum RateDataError {
    #[error("Singatures cache error: {0}")]
    SignaturesCacheError(#[from] SignaturesCacheError),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
    pub signature: Option<String>,
}

impl RateDataLight {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = vec![
            Token::String(self.symbol.clone()),
            Token::Uint(self.rate.into()),
            Token::Uint(self.decimals.into()),
            Token::Uint(self.timestamp.into()),
        ];

        encode_packed(&raw_data).expect("tokens should be valid")
    }

    pub async fn sign(&mut self) -> Result<(), RateDataError> {
        let sign_data = self.encode_packed();

        self.signature = Some(hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        ));

        Ok(())
    }
}
