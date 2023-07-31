#[cfg(test)]
mod balances;
#[cfg(test)]
mod default_pairs;
#[cfg(test)]
mod whitelist;

mod utils;

use candid::{CandidType, Decode, IDLArgs};
use serde::Deserialize;
use std::process::Command;

const TEST_IDENTITY1: &str = "dfx_test_key";
const TEST_IDENTITY2: &str = "dfx_test_key2";
const REPLICA_HEALTHY_STATUS: &str = "\"replica_health_status\": \"healthy\"";

pub fn canonical_sybil_dir() -> String {
    std::fs::canonicalize("..")
        .expect("should be able to canonicalize sybil dir")
        .to_str()
        .expect("should be able to convert canonical path to str")
        .into()
}

pub fn is_dfx_installed() -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        panic!("This test can only be run on Linux");
    }

    let output = Command::new("dfx")
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to execute command: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8(output.stderr).unwrap());
    }

    Ok(())
}

pub fn is_sybil_canister(network: &str) -> Result<(), String> {
    let output = Command::new("dfx")
        .current_dir(canonical_sybil_dir())
        .arg("canister")
        .arg("id")
        .arg("sybil")
        .arg("--network")
        .arg(network)
        .output()
        .map_err(|e| format!("failed to execute command: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8(output.stderr).unwrap());
    }

    Ok(())
}

pub fn is_dfx_replica_up() -> Result<(), String> {
    let output = Command::new("dfx")
        .arg("ping")
        .output()
        .expect("failed to execute method");

    if !output.status.success() {
        panic!("{:?}", String::from_utf8(output.stderr).unwrap());
    }

    let data = String::from_utf8(output.stdout).expect("invalid utf8 string in stdout");

    assert!(
        data.contains(REPLICA_HEALTHY_STATUS),
        "replica is not healthy"
    );

    Ok(())
}

pub fn is_test_identities_exist() -> Result<(), String> {
    let output = Command::new("dfx")
        .args(["identity", "list"])
        .output()
        .map_err(|e| format!("failed to execute command: {e}"))?;

    if !output.status.success() {
        panic!("{:?}", String::from_utf8(output.stderr).unwrap());
    }

    let data = String::from_utf8(output.stdout).expect("invalid utf8 string in stdout");

    if !data.contains(TEST_IDENTITY1) || !data.contains(TEST_IDENTITY2) {
        panic!("{} and {} should exist", TEST_IDENTITY1, TEST_IDENTITY2);
    }

    Ok(())
}

pub fn clear_state() -> Result<(), String> {
    sybil_execute("clear_state", None)
}

pub fn pre_test() -> Result<(), String> {
    is_dfx_installed()?;
    is_dfx_replica_up()?;
    switch_to_dfx_test_key1();
    is_sybil_canister(&get_network())?;
    is_test_identities_exist()?;

    Ok(())
}

pub fn get_network() -> String {
    std::env::var("NETWORK").unwrap_or_else(|_| "local".to_string())
}

pub fn get_test_address() -> String {
    std::env::var("ADDRESS").expect("ADDRESS should be setted")
}

pub fn stdout_decode<T: CandidType + for<'a> Deserialize<'a>>(stdout: Vec<u8>) -> T {
    let data = String::from_utf8(stdout).expect("invalid utf8 string in stdout");
    let args: IDLArgs = data.parse().expect("failed to parse stdout to IDLArgs");
    Decode!(&args.to_bytes().unwrap(), T).expect("failed to decode stdout")
}

pub fn sybil_execute<T: CandidType + for<'a> Deserialize<'a>>(
    method: &str,
    args: Option<&str>,
) -> T {
    let network = get_network();

    let mut cmd = Command::new("dfx");

    cmd.current_dir(canonical_sybil_dir())
        .args(["canister", "call", "sybil", method]);

    if let Some(args) = args {
        cmd.arg(args);
    }

    cmd.args(["--network", &network]);

    println!("args: {:?}", cmd.get_args());

    let output = cmd.output().expect("failed to execute method");

    if !output.status.success() {
        panic!("{:?}", String::from_utf8(output.stderr).unwrap());
    }

    stdout_decode(output.stdout)
}

pub fn switch_identity(identity: &str) {
    let output = Command::new("dfx")
        .current_dir(canonical_sybil_dir())
        .args(["identity", "use", identity])
        .output()
        .expect("failed to switch identity");

    if !output.status.success() {
        panic!("{:?}", String::from_utf8(output.stderr).unwrap());
    }
}

pub fn switch_to_dfx_test_key1() {
    switch_identity(TEST_IDENTITY1);
}

pub fn switch_to_dfx_test_key2() {
    switch_identity(TEST_IDENTITY2);
}
