use scopeguard::defer;

use crate::{get_test_address, sybil_execute, pre_test, clear_state, switch_to_dfx_test_key2, switch_to_dfx_test_key1};


pub fn add_to_whitelist(address: &str) -> Result<(), String> {
    sybil_execute("add_to_whitelist", Some(address))
}

pub fn remove_from_whitelist(address: &str) -> Result<(), String> {
    sybil_execute("remove_from_whitelist", Some(address))
}

pub fn is_whitelisted(address: &str) -> Result<bool, String> {
    sybil_execute("is_whitelisted", Some(address))
}

pub fn get_whitelist() -> Result<Vec<String>, String> {
    sybil_execute("get_whitelist", None)
}

#[test]
fn test_whitelist_with_valid_data() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    let test_address = get_test_address();

    add_to_whitelist(&test_address)
        .expect("failed to add to whitelist");
    assert_eq!(is_whitelisted(&test_address), Ok(true), "address is not whitelised");
    
    remove_from_whitelist(&test_address)
        .expect("failed to remove from whitelist");
    assert_eq!(is_whitelisted(&test_address), Ok(false), "address is whitelised");

    add_to_whitelist(&test_address)
        .expect("failed to add to whitelist");
    let whitelist = get_whitelist()
        .expect("failed to get whitelist");
    assert!(whitelist.contains(&test_address), "address is not whitelised");
}

#[test]
fn test_whitelist_with_invalid_test_address_error() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    // invalid address
    let invalid_test_address = "invalid-test-address".to_string();
    assert!(add_to_whitelist(&invalid_test_address).unwrap_err().contains("invalid hex"), "should fail with invalid hex");
    assert!(remove_from_whitelist(&invalid_test_address).unwrap_err().contains("invalid hex"), "should fail with invalid hex");
    assert!(is_whitelisted(&invalid_test_address).unwrap_err().contains("invalid hex"), "should fail with invalid hex");
}

#[test]
fn test_whitelist_with_is_whitelisted_error() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    let test_address = get_test_address();
    add_to_whitelist(&test_address)
        .expect("failed to add to whitelist");
    assert!(add_to_whitelist(&test_address).unwrap_err().contains("Address is already whitelisted"), "should fail with user is already whitelisted");
}

#[test]
fn test_whitelist_with_not_whitelisted_error() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    let test_address = get_test_address();
    assert!(remove_from_whitelist(&test_address).unwrap_err().contains("Address is not whitelisted"), "should fail with user is not whitelisted");
}

#[test]
fn test_whitelist_not_controller_error() {
    pre_test()
        .expect("failed to run pre tests");
    defer!(clear_state().expect("failed to clear state"));

    let test_address = get_test_address();
    switch_to_dfx_test_key2();
    assert!(add_to_whitelist(&test_address).unwrap_err().contains("Caller is not a controller"), "should fail with not controller");
    assert!(remove_from_whitelist(&test_address).unwrap_err().contains("Caller is not a controller"), "should fail with not controller");
    assert!(get_whitelist().unwrap_err().contains("Caller is not a controller"), "should fail with not controller");
    switch_to_dfx_test_key1();
}
