use std::str::FromStr;

use anyhow::anyhow;
use siwe::{Message, ParseError, VerificationError, VerificationOpts};
use thiserror::Error;
use time_rs::{error::ComponentRange as TimeError, OffsetDateTime};

use super::{
    address::{self, AddressError},
    convertion::u64_to_i64,
    time,
};
use crate::{clone_with_state, types::Address};

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
    if !clone_with_state!(mock) {
        let sig = hex::decode(sig).map_err(|e| anyhow!("invalid signature: {}", e))?;
        let opts = VerificationOpts {
            timestamp: Some(OffsetDateTime::from_unix_timestamp(u64_to_i64(
                time::in_seconds(),
            ))?),
            ..Default::default()
        };

        msg.verify(&sig, &opts).await?;
    }

    Ok(address::from_str(&hex::encode(msg.address))?)
}
