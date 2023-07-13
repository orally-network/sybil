use candid::Principal;
use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::balances::BalancesCfg;

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Cfg {
    pub exchange_rate_canister: Principal,
    pub mock: bool,
    pub key_name: String,
    pub balances_cfg: BalancesCfg,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct UpdateCfg {
    pub exchange_rate_canister: Option<Principal>,
    pub mock: Option<bool>,
    pub key_name: Option<String>,
    pub balances_cfg: Option<BalancesCfg>,
}
