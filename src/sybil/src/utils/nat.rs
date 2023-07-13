use candid::Nat;

use ic_web3_rs::types::U256;
use num_bigint::BigUint;

pub fn to_u64(nat: &Nat) -> u64 {
    let nat_digits = nat.0.to_u64_digits();

    let mut number: u64 = 0;
    if !nat_digits.is_empty() {
        number = *nat_digits
            .last()
            .expect("nat should have at least one digit");
    }

    number
}

pub fn to_u256(nat: &Nat) -> U256 {
    U256::from_big_endian(&nat.0.to_bytes_be())
}

pub fn from_u256(u256: &U256) -> Nat {
    let mut buf = Vec::with_capacity(32);

    for i in u256.0.iter().rev() {
        buf.extend(i.to_be_bytes());
    }

    Nat(BigUint::from_bytes_be(&buf))
}
