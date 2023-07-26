use std::thread::sleep;
use std::time::Duration;

use scopeguard::defer;
use candid::CandidType;
use serde::Deserialize;

use crate::{pre_test, clear_state, sybil_execute};

pub const PAIR_ID: &str = "ETH/USD";
pub const PAIR_UPDATE_FREQUENCY_CANDID: &str = "5:nat";
pub const PAIR_UPDATE_FREQUENCY: u64 = 5;
pub const PAIR_DECIMALS_CANDID: &str = "9:nat";
pub const PAIR_DECIMALS: u64 = 9;

#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Source {
    pub uri: String,
    pub resolver: String,
    pub expected_bytes: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub enum PairType {
    Custom {
        sources: Vec<Source>,
    },
    Default,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct PairStatus {
    last_update: u64,
    updated_counter: u64,
    requests_counter: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Pair {
    pub id: String,
    pub pair_type: PairType,
    pub update_freq: u64,
    pub decimals: u64,
    pub status: PairStatus,
    pub owner: String,
}

pub fn create_default_pair() -> Result<(), String> {
    sybil_execute(
        "create_default_pair",
        Some(&format!("(record {{pair_id=\"{PAIR_ID}\"; update_freq={PAIR_UPDATE_FREQUENCY_CANDID}; decimals={PAIR_DECIMALS_CANDID}}})"))
    )
}

pub fn get_asset_data() -> Result<RateDataLight, String> {
    sybil_execute("get_asset_data", Some(&format!("(\"{PAIR_ID}\")")))
}

pub fn get_asset_data_with_proof() -> Result<RateDataLight, String> {
    sybil_execute("get_asset_data_with_proof", Some(&format!("(\"{PAIR_ID}\")")))
}

pub fn is_pair_exists() -> bool {
    sybil_execute("is_pair_exists", Some(&format!("(\"{PAIR_ID}\")")))
}

pub fn remove_default_pair() -> Result<(), String> {
    sybil_execute("remove_default_pair", Some(&format!("(\"{PAIR_ID}\")")))
}

pub fn get_pairs() -> Vec<Pair> {
    sybil_execute("get_pairs", None)
}

#[test]
fn test_default_pair_with_valid_data() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    create_default_pair()
        .expect("failed to create default pair");

    let rate = get_asset_data()
        .expect("failed to get asset data");

    assert_eq!(rate.symbol, PAIR_ID, "invalid symbol");
    assert_eq!(rate.decimals, PAIR_DECIMALS, "invalid decimals");
    assert_eq!(rate.signature, None, "signature should be None");

    let cached_rate = get_asset_data()
        .expect("failed to get asset data");
    assert_eq!(rate.timestamp, cached_rate.timestamp, "rate should be cached");

    sleep(Duration::from_secs(PAIR_UPDATE_FREQUENCY + 1));

    let updated_rate = get_asset_data()
        .expect("failed to get asset data");
    assert_ne!(rate.timestamp, updated_rate.timestamp, "rate should be updated");

    assert!(is_pair_exists(), "pair should exists");

    let signed_rate = get_asset_data_with_proof()
        .expect("failed to get asset data with proof");

    assert!(signed_rate.signature.is_some(), "signature should be Some");

    let pairs = get_pairs();
    assert_eq!(pairs.len(), 1, "invalid pairs count");  

    remove_default_pair()
        .expect("failed to remove default pair");

    assert!(!is_pair_exists(), "pair should not exists");
}
