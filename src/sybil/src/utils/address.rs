use sha3::{Digest, Keccak256};
use thiserror::Error;

use crate::types::Address;

const PREFIX: &str = "0x";
const ADDRESS_LENGTH: usize = 40;

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Invalid hex")]
    InvalidHex,
}

pub fn from_str(address: &str) -> Result<Address, AddressError> {
    let trimmed = address.trim_start_matches(PREFIX);
    if !is_valid(trimmed) {
        return Err(AddressError::InvalidHex);
    }

    let stripped = trimmed.to_ascii_lowercase();

    let mut hasher = Keccak256::new();
    hasher.update(stripped);
    let hash_vec = hasher.finalize().to_vec();
    let hash = hex::encode(hash_vec);

    let mut checksum = String::new();

    for (pos, char) in hash.chars().enumerate() {
        if pos > 39 {
            break;
        }
        if u32::from_str_radix(&char.to_string()[..], 16).expect("should be valid number") > 7 {
            checksum.push_str(&trimmed[pos..pos + 1].to_ascii_uppercase());
        } else {
            checksum.push_str(&trimmed[pos..pos + 1].to_ascii_lowercase());
        }
    }
    Ok(format!("0x{checksum}"))
}

pub fn is_valid(address: &str) -> bool {
    if address.len() != ADDRESS_LENGTH {
        return false;
    }

    address.chars().all(|c| c.is_ascii_hexdigit())
}