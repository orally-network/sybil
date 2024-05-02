use ic_cdk::{query, update};

use crate::types::api_keys::{APIKeys, APIKeysError, User};
use crate::utils::{siwe, validate_caller};

#[update]
pub async fn get_api_key(msg: String, sig: String) -> Result<String, String> {
    _get_api_key(msg, sig)
        .await
        .map_err(|e| format!("cannot get api key: {}", e))
}

#[inline(always)]
async fn _get_api_key(msg: String, sig: String) -> Result<String, APIKeysError> {
    let caller = siwe::recover(&msg, &sig).await?;

    Ok(APIKeys::generate_new(caller).await?)
}

#[query]
pub async fn get_user_api_keys(address: String) -> Result<Option<Vec<String>>, String> {
    _get_user_api_keys(address)
        .await
        .map_err(|e| format!("cannot get user api keys: {}", e))
}

#[inline(always)]
async fn _get_user_api_keys(address: String) -> Result<Option<Vec<String>>, APIKeysError> {
    validate_caller()?;
    Ok(APIKeys::get_user_api_keys(&address)?)
}

#[query]
pub async fn get_user_by_key(key: String) -> Result<Option<User>, String> {
    _get_user_by_key(key)
        .await
        .map_err(|e| format!("cannot get user by key: {}", e))
}

#[inline(always)]
async fn _get_user_by_key(key: String) -> Result<Option<User>, APIKeysError> {
    validate_caller()?;
    Ok(APIKeys::get_user_by_key(&key)?)
}

#[update]
pub async fn revoke_keys(address: String) -> Result<(), String> {
    _revoke_keys(address)
        .await
        .map_err(|e| format!("cannot revoke keys: {}", e))
}

#[inline(always)]
async fn _revoke_keys(address: String) -> Result<(), APIKeysError> {
    validate_caller()?;
    Ok(APIKeys::revoke_keys(&address)?)
}

#[update]
pub async fn revoke_key(key: String) -> Result<(), String> {
    _revoke_keys(key)
        .await
        .map_err(|e| format!("cannot revoke key: {}", e))
}

#[inline]
pub async fn _revoke_key(key: String) -> Result<(), APIKeysError> {
    validate_caller()?;
    APIKeys::revoke_key(key);
    Ok(())
}

#[update]
pub async fn update_request_limit(
    key: String,
    new_limit: u64,
    msg: String,
    sig: String,
) -> Result<(), String> {
    _update_request_limit(key, new_limit, msg, sig)
        .await
        .map_err(|e| format!("cannot update request limit: {}", e))
}

#[inline]
pub async fn _update_request_limit(
    key: String,
    new_limit: u64,
    msg: String,
    sig: String,
) -> Result<(), APIKeysError> {
    let caller = siwe::recover(&msg, &sig).await?;

    APIKeys::update_request_limit(caller, key, new_limit)
}

#[update]
pub async fn update_request_limit_by_domain(
    key: String,
    new_limit: u64,
    msg: String,
    sig: String,
) -> Result<(), String> {
    _update_request_limit_by_domain(key, new_limit, msg, sig)
        .await
        .map_err(|e| format!("cannot update request limit: {}", e))
}

#[inline]
pub async fn _update_request_limit_by_domain(
    key: String,
    new_limit: u64,
    msg: String,
    sig: String,
) -> Result<(), APIKeysError> {
    let caller = siwe::recover(&msg, &sig).await?;

    APIKeys::update_request_limit_by_domain(caller, key, new_limit)
}

#[update]
pub async fn update_free_request_limit(new_limit: u64) -> Result<(), String> {
    _update_free_request_limit(new_limit)
        .await
        .map_err(|e| format!("cannot update request limit: {}", e))
}

#[inline]
pub async fn _update_free_request_limit(new_limit: u64) -> Result<(), APIKeysError> {
    validate_caller()?;
    APIKeys::update_free_request_limit(new_limit);

    Ok(())
}

#[query]
pub async fn get_api_keys() -> Result<APIKeys, String> {
    _get_api_keys()
        .await
        .map_err(|e| format!("cannot get api keys: {}", e))
}

#[inline(always)]
async fn _get_api_keys() -> Result<APIKeys, APIKeysError> {
    validate_caller()?;
    Ok(APIKeys::get_api_keys())
}

#[update]
pub async fn ban_domain(
    key: String,
    domain: String,
    msg: String,
    sig: String,
) -> Result<(), String> {
    _ban_domain(key, domain, msg, sig)
        .await
        .map_err(|e| format!("cannot update request limit: {}", e))
}

#[inline]
pub async fn _ban_domain(
    key: String,
    domain: String,
    msg: String,
    sig: String,
) -> Result<(), APIKeysError> {
    let caller = siwe::recover(&msg, &sig).await?;

    APIKeys::ban_domain(caller, key, domain)
}

#[update]
pub async fn allow_domain(
    key: String,
    domain: String,
    msg: String,
    sig: String,
) -> Result<(), String> {
    _allow_domain(key, domain, msg, sig)
        .await
        .map_err(|e| format!("cannot update request limit: {}", e))
}

#[inline]
pub async fn _allow_domain(
    key: String,
    domain: String,
    msg: String,
    sig: String,
) -> Result<(), APIKeysError> {
    let caller = siwe::recover(&msg, &sig).await?;

    APIKeys::allow_domain(caller, key, domain)
}
