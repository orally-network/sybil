use candid::Principal;
use ic_cdk::api::{
    call::{call_with_payment, CallResult},
    management_canister::ecdsa::{SignWithEcdsaArgument, SignWithEcdsaResponse},
};
use ic_web3_rs::{ic::recover_address, types::H160};
use thiserror::Error;

const ECDSA_SIGN_CYCLES: u64 = 23_000_000_000;

#[derive(Error, Debug)]
pub enum SignatureError {
    #[error("Invalid signature format")]
    InvalidSignatureFormat,
}

pub fn get_eth_v(
    signature: &[u8],
    message: &[u8],
    public_key: &H160,
) -> Result<u8, SignatureError> {
    let pub_key = hex::encode(public_key);
    if pub_key == recover_address(message.to_vec(), signature.to_vec(), 0) {
        return Ok(27);
    }

    if pub_key == recover_address(message.to_vec(), signature.to_vec(), 1) {
        return Ok(28);
    }

    Err(SignatureError::InvalidSignatureFormat)
}

pub async fn sign(args: SignWithEcdsaArgument) -> CallResult<(SignWithEcdsaResponse,)> {
    call_with_payment(
        Principal::management_canister(),
        "sign_with_ecdsa",
        (args,),
        ECDSA_SIGN_CYCLES,
    )
    .await
}
