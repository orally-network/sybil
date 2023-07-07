use candid::Nat;

pub fn to_u64(nat: &Nat) -> u64 {
    let nat_digits = nat.0.to_u64_digits();
    let mut number: u64 = 0;
    if !nat_digits.is_empty() {
        number = *nat_digits.last().expect("nat should be a number");
    }
    number
}
