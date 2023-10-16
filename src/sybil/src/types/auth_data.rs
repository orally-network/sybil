use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use ic_web3_rs::ethabi::Token;
use thiserror::Error;
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;

use crate::utils::encoding::encode_packed;

use super::cache::{SignaturesCache, SignaturesCacheError};

#[derive(Error, Debug)]
pub enum AuthDataError {
    #[error("Singatures cache error: {0}")]
    SignaturesCacheError(#[from] SignaturesCacheError),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct AuthData {
    pub user_id: String,
    pub access_token: String,
    pub service: String,
    pub timestamp: u64,
    pub signature: String,
}

impl AuthData {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = vec![
            Token::String(self.user_id.clone()),
            Token::String(self.access_token.clone()),
            Token::String(self.service.clone()),
            Token::Uint(self.timestamp.into()),
        ];
        
        encode_packed(&raw_data).expect("tokens should be valid")
    }
    
    pub async fn sign(&mut self) -> Result<(), AuthDataError> {
        let sign_data = self.encode_packed();
        
        self.signature = hex::encode(
            SignaturesCache::eth_sign_with_access(&sign_data).await?,
        );
        
        Ok(())
    }
}
