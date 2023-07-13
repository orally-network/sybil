pub mod address;
pub mod canister;
pub mod convertion;
pub mod encoding;
pub mod macros;
pub mod nat;
pub mod processors;
pub mod signature;
pub mod siwe;
pub mod time;
pub mod validation;
pub mod vec;
pub mod web3;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CallerError {
    #[error("Caller is not a controller")]
    CallerIsNotController,
}

pub fn validate_caller() -> Result<(), CallerError> {
    if ic_cdk::api::is_controller(&ic_cdk::caller()) {
        return Ok(());
    }

    Err(CallerError::CallerIsNotController)
}
