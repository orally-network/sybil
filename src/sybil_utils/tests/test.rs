use sybil_utils::cycles_count;

macro_rules! log {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

mod ic_cdk {
    pub mod api {
        pub fn canister_balance() -> i32 {
            100
        }
    }
}

#[cycles_count]
fn test_func(a: i32) -> i32 {
    log!("TEST FUNC");
    a + 1
}

#[test]
fn test_aboba() {
    assert_eq!(test_func(1), 2);
}
