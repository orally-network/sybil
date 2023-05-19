use std::str::FromStr;

use anyhow::{anyhow, Result};

use candid::Principal;
use ic_cdk::export::{
    candid::{CandidType, Nat},
    serde::{Deserialize, Serialize},
};

use crate::STATE;

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum DepositType {
    Erc20,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct DepositRequest {
    pub amount: Nat,
    pub taxpayer: String,
    pub deposit_type: DepositType,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum TextResult {
    Ok(()),
    Err(String),
}

pub async fn deposit(req: DepositRequest) -> Result<(), String> {
    _deposit(req).await.map_err(|e| e.to_string())
}

async fn _deposit(req: DepositRequest) -> Result<()> {
    let treasurer_canister = STATE.with(|state| state.borrow().treasurer_canister.clone());

    let treasurer_canister = Principal::from_str(&treasurer_canister)?;

    let (result,): (TextResult,) = ic_cdk::call(treasurer_canister, "deposit", (req,))
        .await
        .map_err(|(code, msg)| anyhow!("{:?}: {}", code, msg))?;

    match result {
        TextResult::Ok(_) => Ok(()),
        TextResult::Err(err) => Err(anyhow!(err)),
    }
}
