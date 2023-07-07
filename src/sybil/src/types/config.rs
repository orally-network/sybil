use candid::Principal;
use ic_cdk::export::{
    candid::{CandidType, Nat},
    serde::{Deserialize, Serialize},
};

use super::balances::BalancesCfg;

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Cfg {
    pub exchange_rate_canister: Principal,
    pub siwe_signer_canister: Principal,
    pub treasurer_canister: Principal,
    pub key_name: String,
    pub cache_expiration: Nat,
    pub cost_per_execution: Nat,
    pub balances_cfg: BalancesCfg,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct UpdateCfg {
    pub exchange_rate_canister: Option<Principal>,
    pub siwe_signer_canister: Option<Principal>,
    pub treasurer_canister: Option<Principal>,
    pub key_name: Option<String>,
    pub cache_expiration: Option<Nat>,
    pub cost_per_execution: Option<Nat>,
    pub balances_cfg: Option<BalancesCfg>,
}
