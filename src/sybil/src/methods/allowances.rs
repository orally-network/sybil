use ic_cdk::update;

use crate::{log, types::allowances::Allowances, utils::siwe};
use anyhow::Result;

/// Allow smartcontract use funds from the user's balance
///
/// # Arguments
///
/// * `grantee` - Origin or Referer http header who is about to be granted
/// * `msg` - SIWE message, For more information, refer to the [SIWE message specification](https://eips.ethereum.org/EIPS/eip-4361)
/// * `sig` - SIWE signature, For more information, refer to the [SIWE message specification](https://eips.ethereum.org/EIPS/eip-4361)
///
/// # Returns
///
/// Returns a result that can contain an error message

#[update]
pub async fn grant(grantee: String, msg: String, sig: String) -> Result<(), String> {
    let user = siwe::recover(&msg, &sig).await.map_err(|e| e.to_string())?;

    Allowances::grant(grantee.clone(), user.clone()).map_err(|err| err.to_string())?;

    log!("[ALLOWANCE] {user} allowed {grantee} to use his balance");
    Ok(())
}

/// Restrict smartcontract from using funds from the user's balance
///
/// # Arguments
///
/// * `grantee` - Origin or Referer http header who is granted
/// * `msg` - SIWE message, For more information, refer to the [SIWE message specification](https://eips.ethereum.org/EIPS/eip-4361)
/// * `sig` - SIWE signature, For more information, refer to the [SIWE message specification](https://eips.ethereum.org/EIPS/eip-4361)
///
/// # Returns
///
/// Returns a result that can contain an error message

#[update]
pub async fn restrict(grantee: String, msg: String, sig: String) -> Result<(), String> {
    let user = siwe::recover(&msg, &sig).await.map_err(|e| e.to_string())?;

    Allowances::restrict(grantee.clone(), user.clone()).map_err(|err| err.to_string())?;

    log!("[ALLOWANCE] {user} restricted {grantee} from using his balance");
    Ok(())
}
