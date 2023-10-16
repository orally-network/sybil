use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use ic_web3_rs::ethabi::Token;
use thiserror::Error;
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
use jsonptr::{Pointer, Resolve};
use serde_json::Value;
use crate::log;

use crate::utils::encoding::encode_packed;

use super::cache::{SignaturesCache, SignaturesCacheError, HttpCache, HttpCacheError};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Service {
    pub resolver: String,
    pub name: String,
    pub verify_url: String,
}

impl Service {
    pub async fn verify(&self, access_token: String, user_id: String) -> Result<bool, HttpCacheError> {
        let req = CanisterHttpRequestArgument {
            url: self.verify_url.clone(),
            // max_response_bytes: Some(self.expected_bytes),
            ..Default::default()
        };
    
        let (response, _) = HttpCache::request_with_access(&req, 1).await?;
    
        let ptr = Pointer::try_from(self.resolver.clone())
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?;
    
        let data = serde_json::from_slice::<Value>(&response.body)?;
    
        let res_user_id = data
            .resolve(&ptr)
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?
            .as_string()
            .ok_or(HttpCacheError::InvalidResponseBodyResolver(
                "value is not a string".into(),
            ))?;
        
        log!("[Service::verify] access_token: {access_token}, user_id: {user_id}, res_user_id: {res_user_id}");
    
        Ok(res_user_id == user_id)
    }
}
