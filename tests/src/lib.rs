#[cfg(test)]
mod whitelist;

use std::process::Command;
use candid::{CandidType, IDLArgs, Decode};
use serde::{Deserialize, Serialize};

pub const PAIR_ID: &str = "ETH/USD";
pub const PAIR_UPDATE_FREQUENCY: &str = "360:nat";
pub const PAIR_DECIMALS: &str = "9:nat";

#[derive(Debug, Default, Clone, CandidType, Deserialize, Serialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
    pub signature: Option<String>,
}

pub fn is_dfx_installed() {
    if !cfg!(target_os = "linux") {
        panic!("This test can only be run on Linux");
    }

    Command::new("dfx")
        .arg("--version")
        .status()
        .expect("dfx is not installed");
}

pub fn is_sybil_canister(network: &str) {
    Command::new("dfx")
        .arg("canister")
        .arg("sybil")
        .arg("id")
        .arg("--network")
        .arg(network)
        .status()
        .expect("canister is not sybil");
}

pub fn clear_state() -> Result<RateDataLight, String> {
    sybil_execute("clear_state", "()")
}

pub fn create_default_pair() -> Result<(), String> {
    sybil_execute(
        "create_default_pair",
        &format!("'(record {{pair_id=\"{PAIR_ID}\"; update_freq={PAIR_UPDATE_FREQUENCY}; decimals={PAIR_DECIMALS}}})'")
    )
}

pub fn get_asset_data() -> Result<RateDataLight, String> {
    sybil_execute("get_asset_data", &format!("'(\"{PAIR_ID}\")'"))
}


pub fn get_network() -> String {
    std::env::var("NETWORK").unwrap_or_else(|_| "local".to_string())
}

pub fn get_test_address() -> String {
    std::env::var("ADDRESS").expect("ADDRESS should be setted")
}

pub fn stdout_decode<T: CandidType+for<'a> Deserialize<'a>>(stdout: Vec<u8>) -> T {
    let data = String::from_utf8(stdout)
        .expect("invalid utf8 string in stdout");
    let args: IDLArgs = data.parse()
        .expect("failed to parse stdout to IDLArgs");
    Decode!(&args.to_bytes().unwrap(), T).expect("failed to decode stdout")
}

pub fn sybil_execute<T: CandidType+for<'a> Deserialize<'a>>(method: &str, args: &str) -> T {
    let network = get_network();

    let output = Command::new("dfx")
        .arg("canister")
        .arg("call")
        .arg("sybil")
        .arg(method)
        .arg(format!("'({})'", args))
        .arg("--network")
        .arg(network)
        .output()
        .expect("failed to execute method");

    assert_eq!(output.status.success(), true, "failed to execute method");
    stdout_decode(output.stdout)
}
