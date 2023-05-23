use ic_cdk::export::{
    Principal, {
        candid::CandidType,
        serde::{Deserialize, Serialize},
    }
};

use super::{custom_pair::CustomPair, rate_data::RateDataLight};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Pair {
    pub id: String,
    pub frequency: u64,
    pub last_update: u64,
    pub data: RateDataLight,
}

#[derive(Clone, Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub exchange_rate_canister: String,
    pub proxy_ecdsa_canister: String,
    pub siwe_signer_canister: String,
    pub pairs: Vec<Pair>,
    pub custom_pairs: Vec<CustomPair>,
    pub key_name: String,
    pub cache_expiration: u64,
    pub treasurer_canister: String,
    pub cost_per_execution: u64,
    pub controllers: Vec<Principal>,
}
