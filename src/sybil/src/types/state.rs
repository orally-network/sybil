use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::custom_pair::CustomPair;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Pair {
    pub id: String,
    pub frequency: u64,
}

#[derive(Clone, Default)]
pub struct State {
    pub exchange_rate_canister: String,
    pub proxy_ecdsa_canister: String,
    pub siwe_signer_canister: String,
    pub pairs: Vec<Pair>,
    pub custom_pairs: Vec<CustomPair>,
}
