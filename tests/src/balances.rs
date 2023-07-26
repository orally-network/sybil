// use scopeguard::defer;

// use crate::{sybil_execute, pre_test, clear_state};

// const SIWE_MSG: &str = "service.org wants you to sign in with your Ethereum account:
// 0xE86C4A45C1Da21f8838a1ea26Fc852BD66489ce9


// URI: https://service.org/login
// Version: 1
// Chain ID: 11155111
// Nonce: 00000000
// Issued At: 2023-05-04T18:39:24Z";
// const SIWE_SIG: &str = "fa7b336d271b7ed539b6db3034d57be294ef889b42534fa95689afd0989ab6d27878c837a14ed1b4c3ab6b7052180ce87198934cb7712a81ea413fd8ebb29e8c1c";

// pub fn deposit() -> Result<(), String> {
//     sybil_execute("deposit", None)
// }

// #[test]
// pub fn test_balance_with_valid_data() {
//     pre_test()
//         .expect("failed to run pre tests");
//     defer!(clear_state().expect("failed to clear state"));
// }
