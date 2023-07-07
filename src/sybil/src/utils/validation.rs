use candid::Nat;
use lazy_static::lazy_static;
use regex::Regex;
use validator::ValidationError;

const MIN_UPDATE_FREQ: u64 = 60 * 5;

lazy_static! {
    pub static ref PAIR_ID_REGEX: Regex = Regex::new(r"^\w+/\w+$").expect("invalid regex");
    pub static ref RATE_RESOLVER: Regex = Regex::new(r"^[[\p{L}_][\p{L}\p{N}_]//*]*$").expect("invalid regex");
}

pub fn validate_update_freq(update_freq: &Nat) -> Result<(), ValidationError> {
    if update_freq.clone() < Nat::from(MIN_UPDATE_FREQ) {
        return Err(ValidationError::new("update_freq is lower than 5 minutes"));
    }
    Ok(())
}
