use crate::types::{Seconds, Timestamp};
use ic_cdk::api::{management_canister::main::raw_rand, time};

pub async fn wait(delay: Seconds) {
    let end = in_seconds() + delay;
    while in_seconds() < end {
        let _ = raw_rand().await;
    }
}

#[inline]
pub fn in_seconds() -> Timestamp {
    time() / 1_000_000_000
}
