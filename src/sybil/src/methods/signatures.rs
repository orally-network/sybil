use crate::{utils::validate_caller, SIGNATURES_CACHE};
use anyhow::Result;
use ic_cdk::update;

#[update]
async fn sign_message(message: String) -> Result<String, String> {
    _sign_message(message)
        .await
        .map_err(|e| format!("Failed to sign message: {}", e))
}

#[inline]
async fn _sign_message(message: String) -> Result<String> {
    validate_caller()?;
    let mut cache = SIGNATURES_CACHE.with(|c| c.borrow().clone());
    let signature = cache.eth_sign(message.as_bytes()).await?;

    let signature = hex::encode(signature);

    Ok(signature)
}
