use crate::{get_test_address, sybil_execute};

#[test]
fn test_whitelist_with_valid_data() {
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
    assert_eq!(whitelist.contains(&test_address), true, "address is not whitelised");
}

#[test]
fn test_whitelist_with_invalid_test_address() {
    // let invalid_test_address = "invalid-test-address".to_string();
}

pub fn add_to_whitelist(caller: &str) -> Result<(), String> {
    sybil_execute("add_to_whitelist", &format!("'(\"{caller}\")'"))
}

pub fn remove_from_whitelist(caller: &str) -> Result<(), String> {
    sybil_execute("remove_from_whitelist", &format!("'(\"{caller}\")'"))
}

pub fn is_whitelisted(caller: &str) -> Result<bool, String> {
    sybil_execute("is_whitelisted", &format!("'(\"{caller}\")'"))
}

pub fn get_whitelist() -> Result<Vec<String>, String> {
    sybil_execute("get_whitelist", &format!("'()"))   
}
