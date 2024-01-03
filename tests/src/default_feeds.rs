use std::thread::sleep;
use std::time::Duration;

use candid::CandidType;
use scopeguard::defer;
use serde::Deserialize;

use crate::{clear_state, pre_test, sybil_execute};

pub const FEED_ID: &str = "ETH/USD";
pub const FEED_UPDATE_FREQUENCY_CANDID: &str = "5:nat";
pub const FEED_UPDATE_FREQUENCY: u64 = 5;
pub const FEED_DECIMALS_CANDID: &str = "9:nat";
pub const FEED_DECIMALS: u64 = 9;

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
pub enum FeedType {
    Custom { sources: Vec<Source> },
    Default,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct FeedStatus {
    last_update: u64,
    updated_counter: u64,
    requests_counter: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Feed {
    pub id: String,
    pub feed_type: FeedType,
    pub update_freq: u64,
    pub decimals: u64,
    pub status: FeedStatus,
    pub owner: String,
}

pub fn create_default_feed() -> Result<(), String> {
    sybil_execute(
        "create_default_feed",
        Some(&format!("(record {{feed_id=\"{FEED_ID}\"; update_freq={FEED_UPDATE_FREQUENCY_CANDID}; decimals={FEED_DECIMALS_CANDID}}})"))
    )
}

pub fn get_asset_data() -> Result<RateDataLight, String> {
    sybil_execute("get_asset_data", Some(&format!("(\"{FEED_ID}\")")))
}

pub fn get_asset_data_with_proof() -> Result<RateDataLight, String> {
    sybil_execute(
        "get_asset_data_with_proof",
        Some(&format!("(\"{FEED_ID}\")")),
    )
}

pub fn is_feed_exists() -> bool {
    sybil_execute("is_feed_exists", Some(&format!("(\"{FEED_ID}\")")))
}

pub fn remove_default_feed() -> Result<(), String> {
    sybil_execute("remove_default_feed", Some(&format!("(\"{FEED_ID}\")")))
}

pub fn get_feeds() -> Vec<Feed> {
    sybil_execute("get_feeds", None)
}

#[test]
fn test_default_feed_with_valid_data() {
    pre_test().expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    create_default_feed().expect("failed to create default feed");

    let rate = get_asset_data().expect("failed to get asset data");

    assert_eq!(rate.symbol, FEED_ID, "invalid symbol");
    assert_eq!(rate.decimals, FEED_DECIMALS, "invalid decimals");
    assert_eq!(rate.signature, None, "signature should be None");

    let cached_rate = get_asset_data().expect("failed to get asset data");
    assert_eq!(
        rate.timestamp, cached_rate.timestamp,
        "rate should be cached"
    );

    sleep(Duration::from_secs(FEED_UPDATE_FREQUENCY + 1));

    let updated_rate = get_asset_data().expect("failed to get asset data");
    assert_ne!(
        rate.timestamp, updated_rate.timestamp,
        "rate should be updated"
    );

    assert!(is_feed_exists(), "feed should exists");

    let signed_rate = get_asset_data_with_proof().expect("failed to get asset data with proof");

    assert!(signed_rate.signature.is_some(), "signature should be Some");

    let feeds = get_feeds();
    assert_eq!(feeds.len(), 1, "invalid feeds count");

    remove_default_feed().expect("failed to remove default feed");

    assert!(!is_feed_exists(), "feed should not exists");
}
