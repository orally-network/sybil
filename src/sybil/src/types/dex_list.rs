use std::collections::HashMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::STATE;

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct DEX {
    pub token0_address: String,
    pub token0_decimals: u32,
    pub token0_symbol: String,
    pub token1_address: String,
    pub token1_decimals: u32,
    pub token1_symbol: String,
}

#[derive(Clone, CandidType, Serialize, Deserialize, Debug, Default)]
pub struct DEXList(pub HashMap<String, DEX>);

impl DEXList {
    pub fn add_dex(dex_address: String, dex: DEX) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            state.dex_list.0.insert(dex_address, dex)
        });
    }

    pub fn get_dex(dex_address: &str) -> Option<DEX> {
        STATE.with(|state| {
            let state = state.borrow();
            state.dex_list.0.get(dex_address).cloned()
        })
    }

    pub fn remove_dex(dex_address: &str) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dex_list.0.remove(dex_address);
        });
    }
}
