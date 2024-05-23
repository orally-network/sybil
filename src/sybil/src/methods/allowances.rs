use std::collections::HashMap;

use futures::TryFutureExt;
use ic_cdk::{query, update};

use crate::{
    log,
    types::allowances::{Allowance, Allowances, AllowancesError},
    utils::siwe,
    STATE,
};
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

    Allowances::grant(grantee.clone(), user.clone());

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

    let allowance_user = Allowances::get_allowance_by_domain(&grantee)
        .ok_or("Allowance not found")?
        .grantor_address;

    log!("Allowance user: {allowance_user}, User: {user}");

    if allowance_user != user {
        return Err("You are not allowed to restrict this domain".to_string());
    }

    Allowances::restrict(grantee.clone());

    log!("[ALLOWANCE] {user} restricted {grantee} from using his balance");
    Ok(())
}

/// Returns a list of domains that are allowed to use the user's balance
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
///
#[query]
pub async fn get_allowed_domains(
    msg: String,
    sig: String,
) -> Result<HashMap<String, Allowance>, String> {
    _get_allowed_domains(msg, sig)
        .map_err(|err| err.to_string())
        .await
}

#[inline]
pub async fn _get_allowed_domains(
    msg: String,
    sig: String,
) -> Result<HashMap<String, Allowance>, AllowancesError> {
    let user = siwe::recover(&msg, &sig).await.unwrap();

    let allowed_domains = Allowances::get_user_allowed_domains(&user)?;

    Ok(allowed_domains
        .into_iter()
        .map(|domain| {
            let allowance = Allowances::get_allowance_by_domain(&domain).unwrap();
            (domain, allowance)
        })
        .collect())
}
