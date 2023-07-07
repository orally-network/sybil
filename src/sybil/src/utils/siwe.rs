use std::str::FromStr;

use anyhow::anyhow;
use siwe::{Message, VerificationOpts, ParseError, VerificationError};
use time_rs::{OffsetDateTime, error::ComponentRange as TimeError};
use thiserror::Error;

use super::{time, convertion::u64_to_i64, address::{self, AddressError}};
use crate::types::Address;

#[derive(Error, Debug)]
pub enum SIWEError {
    #[error("invalid message")]
    InvalidMessage(#[from] ParseError),
    #[error("invalid timestamp")]
    InvalidTimestamp(#[from] TimeError),
    #[error("error: {0}")]
    Other(#[from] anyhow::Error),
    #[error("invalid signature")]
    InvalidSignature(#[from] VerificationError),
    #[error("invalid address")]
    InvalidAddress(#[from] AddressError),
}

pub async fn recover(msg: &str, sig: &str) -> Result<Address, SIWEError> {
    let msg = Message::from_str(msg)?;
    let sig = hex::decode(sig)
        .map_err(|e| anyhow!("invalid signature: {}", e))?;
    let opts = VerificationOpts {
        timestamp: Some(OffsetDateTime::from_unix_timestamp(u64_to_i64(time::in_seconds()))?),
        ..Default::default()
    };

    msg.verify(&sig, &opts).await?;

    Ok(address::from_str(&hex::encode(msg.address))?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {   
        println!("[U64] max: {}, min: {}", u64::MAX, u64::MIN);
        println!("[I64] max: {}, min: {}", i64::MAX, i64::MIN);
    }
}
