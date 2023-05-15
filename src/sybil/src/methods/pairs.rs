use ic_cdk::{export::candid::Nat, query, update};

use crate::{types::state::Pair, utils::nat_to_u64, STATE};

#[update]
pub fn add_pair(pair_id: String, frequency: Nat) -> Pair {
    let pair = Pair {
        id: pair_id,
        frequency: nat_to_u64(frequency),
    };

    STATE.with(|state| {
        state.borrow_mut().pairs.push(pair.clone());
    });

    pair
}

#[update]
pub fn remove_pair(pair_id: String) {
    STATE.with(|state| {
        let pairs = &mut state.borrow_mut().pairs;
        if let Some(index) = pairs.iter().position(|pair| pair.id == pair_id) {
            pairs.remove(index);
        }
    });
}

#[query]
pub fn get_pairs() -> Vec<Pair> {
    STATE.with(|state| state.borrow().pairs.clone())
}
