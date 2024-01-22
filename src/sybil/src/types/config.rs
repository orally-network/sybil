use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

use super::balances::BalancesCfg;

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Cfg {
    pub exchange_rate_canister: Principal,
    pub fallback_xrc: Principal,
    pub rpc_wrapper: String,
    pub mock: bool,
    pub key_name: String,
    pub balances_cfg: BalancesCfg,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct UpdateCfg {
    pub exchange_rate_canister: Option<Principal>,
    pub fallback_xrc: Option<Principal>,
    pub rpc_wrapper: Option<String>,
    pub mock: Option<bool>,
    pub key_name: Option<String>,
    pub balances_cfg: Option<BalancesCfg>,
}
