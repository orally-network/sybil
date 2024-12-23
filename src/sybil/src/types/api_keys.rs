use std::collections::{HashMap, HashSet};

use candid::CandidType;
use ic_cdk::api::management_canister::main::raw_rand;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time_rs::OffsetDateTime;

use crate::utils::siwe::SiweError;
use crate::utils::time::in_seconds;
use crate::utils::{address, CallerError};
use crate::STATE;

const HEX_API_KEYS_LEN: usize = 32;
const DEFAULT_REQUEST_LIMIT: u64 = 100_000;
const DEFAULT_REQUEST_BY_DOMAIN_LIMIT: u64 = 10_000;
const DEFAULT_FREE_REQUEST_LIMIT: u64 = 0;

#[derive(Error, Debug)]
pub enum APIKeysError {
    #[error("Failed to get random bytes: {0}")]
    FailedToGetRandomBytes(String),
    #[error("SIWE error: {0}")]
    SiweError(#[from] SiweError),
    #[error("Address error: {0}")]
    AddressError(#[from] address::AddressError),
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
    #[error("Invalid api key")]
    InvalidKey,
    #[error("Limit exceeded")]
    LimitExceeded,
    #[error("Address is not an owner of the key")]
    InvalidOwner,
    #[error("Now Allowed")]
    NotAllowed,
}

#[derive(Default, Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct User {
    pub address: String,
    pub request_count: u64,
    pub request_count_today: u64,
    pub request_count_per_method: HashMap<String, u64>,
    pub request_count_per_domain: HashMap<String, u64>,
    pub banned_domains: HashSet<String>,
    pub allowed_domains: HashSet<String>,
    pub is_public: bool, // if true, everyone except banned domains can use this key, if false, only allowed domains can use this key
    pub last_request: u64, // timestamp of the last request
    pub request_limit_by_domain: u64,
    pub request_limit: u64,
}

impl User {
    pub fn increment_request_count(&mut self) {
        self.request_count += 1;
        self.request_count_today += 1;
        self.last_request = in_seconds();
    }

    pub fn decrease_request_count(&mut self) {
        self.request_count -= 1;
        self.request_count_today -= 1;
        self.last_request = in_seconds();
    }

    pub fn increment_request_count_by_method(&mut self, method: &str) {
        *self
            .request_count_per_method
            .entry(method.to_string())
            .or_default() += 1;
        self.last_request = in_seconds();
    }

    pub fn decrease_request_count_by_method(&mut self, method: &str) {
        *self
            .request_count_per_method
            .entry(method.to_string())
            .or_default() -= 1;
        self.last_request = in_seconds();
    }

    pub fn increment_request_count_by_domain(&mut self, domain: &str) {
        *self
            .request_count_per_domain
            .entry(domain.to_string())
            .or_default() += 1;
        self.last_request = in_seconds();
    }

    pub fn decrease_request_count_by_domain(&mut self, domain: &str) {
        *self
            .request_count_per_domain
            .entry(domain.to_string())
            .or_default() -= 1;
        self.last_request = in_seconds();
    }

    pub fn get_request_count_by_method(&self, method: &str) -> u64 {
        *self.request_count_per_method.get(method).unwrap_or(&0)
    }

    pub fn get_request_count_by_domain(&self, domain: &str) -> u64 {
        *self.request_count_per_domain.get(domain).unwrap_or(&0)
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_count
    }

    pub fn ban_domain(&mut self, domain: String) {
        self.allowed_domains.remove(&domain);
        self.banned_domains.insert(domain);
    }

    pub fn allow_domain(&mut self, domain: String) {
        self.banned_domains.remove(&domain);
        self.allowed_domains.insert(domain);
    }

    pub fn check_restrictions(&self, domain: Option<&str>) -> Result<(), APIKeysError> {
        if self.request_count_today >= self.request_limit {
            return Err(APIKeysError::LimitExceeded);
        }

        if let Some(domain) = domain {
            if self.banned_domains.contains(domain)
                || (!self.is_public && !self.allowed_domains.contains(domain))
            {
                return Err(APIKeysError::NotAllowed);
            }

            if self.get_request_count_by_domain(domain) >= self.request_limit_by_domain {
                return Err(APIKeysError::LimitExceeded);
            }
        }

        Ok(())
    }

    pub fn update_request_count(&mut self) {
        let now = OffsetDateTime::from_unix_timestamp(in_seconds() as i64)
            .unwrap()
            .date();

        let previous_date = OffsetDateTime::from_unix_timestamp(self.last_request as i64)
            .unwrap_or_else(|_| {
                OffsetDateTime::from_unix_timestamp_nanos(self.last_request as i128).unwrap()
            })
            .date();

        if now != previous_date {
            self.request_count_today = 0;
            self.request_count_per_method.clear();
            self.request_count_per_domain.clear();
        }
    }
}

// APIKeys is a map that contains which domains (grantee) are allowed to use which user's (grantor) balance
// grantee domain => grantor public key
#[derive(Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct APIKeys {
    pub keys_to_user: HashMap<String, User>,
    pub user_to_keys: HashMap<String, HashSet<String>>,
    pub free_request_limit: u64,
}

impl Default for APIKeys {
    fn default() -> Self {
        Self {
            keys_to_user: HashMap::new(),
            user_to_keys: HashMap::new(),
            free_request_limit: DEFAULT_FREE_REQUEST_LIMIT,
        }
    }
}

impl APIKeys {
    pub async fn generate_new(address: String) -> Result<String, APIKeysError> {
        // Generating a new key until it is unique
        let key = loop {
            let (bytes,) = raw_rand()
                .await
                .map_err(|(_, err)| APIKeysError::FailedToGetRandomBytes(err))?;

            let key = hex::encode(&bytes[..HEX_API_KEYS_LEN / 2]);

            let is_exist = STATE.with(|state| {
                let state = state.borrow();
                state.api_keys.keys_to_user.contains_key(&key)
            });

            if is_exist {
                continue;
            } else {
                break key;
            }
        };

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.api_keys.keys_to_user.insert(
                key.clone(),
                User {
                    address: address.clone(),
                    request_count: 0,
                    request_count_today: 0,
                    request_count_per_method: HashMap::new(),
                    request_count_per_domain: HashMap::new(),
                    banned_domains: HashSet::new(),
                    allowed_domains: HashSet::new(),
                    is_public: true,
                    last_request: 0,
                    request_limit_by_domain: DEFAULT_REQUEST_BY_DOMAIN_LIMIT,
                    request_limit: DEFAULT_REQUEST_LIMIT,
                },
            );
            state
                .api_keys
                .user_to_keys
                .entry(address)
                .or_default()
                .insert(key.clone())
        });

        Ok(key)
    }

    // Auth key checks if the key is valid and increments the request counter
    // If the request limit is exceeded, it returns an error
    // Returns the user's address and a boolean indicating if the request is free
    pub fn auth_key(
        key: String,
        method: String,
        domain: Option<String>,
    ) -> Result<(String, bool), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let Some(user) = state.api_keys.keys_to_user.get_mut(&key) else {
                return Err(APIKeysError::InvalidKey);
            };

            user.update_request_count();

            user.check_restrictions(domain.as_deref())?;

            user.increment_request_count();
            user.increment_request_count_by_method(&method);
            if let Some(domain) = domain {
                user.increment_request_count_by_domain(&domain);
            }

            Ok((
                user.address.clone(),
                user.get_request_count() <= state.api_keys.free_request_limit,
            ))
        })
    }

    pub fn increment_request_count(
        key: String,
        method: String,
        domain: Option<String>,
    ) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let Some(user) = state.api_keys.keys_to_user.get_mut(&key) else {
                return Err(APIKeysError::InvalidKey);
            };

            user.update_request_count();

            user.check_restrictions(domain.as_deref())?;

            user.increment_request_count();
            user.increment_request_count_by_method(&method);
            if let Some(domain) = domain {
                user.increment_request_count_by_domain(&domain);
            }

            Ok(())
        })
    }

    pub fn decrease_request_count(
        key: String,
        method: String,
        domain: Option<String>,
    ) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let Some(user) = state.api_keys.keys_to_user.get_mut(&key) else {
                return Err(APIKeysError::InvalidKey);
            };

            user.update_request_count();

            user.check_restrictions(domain.as_deref())?;

            user.decrease_request_count();
            user.decrease_request_count_by_method(&method);
            if let Some(domain) = domain {
                user.decrease_request_count_by_domain(&domain);
            }

            Ok(())
        })
    }

    pub fn get_user_api_keys(address: &str) -> Result<Vec<String>, APIKeysError> {
        let address = address::from_str(address)?;

        Ok(STATE.with(|state| {
            let mut state = state.borrow_mut();
            state
                .api_keys
                .user_to_keys
                .entry(address.clone())
                .or_default()
                .clone()
                .into_iter()
                .collect()
        }))
    }

    pub fn get_user_by_key(key: &str) -> Option<User> {
        STATE.with(|state| {
            let state = state.borrow();
            state.api_keys.keys_to_user.get(key).cloned().map(|mut u| {
                u.update_request_count();
                u
            })
        })
    }

    pub fn revoke_keys(address: &str) -> Result<(), APIKeysError> {
        let address = address::from_str(address)?;

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(keys) = state.api_keys.user_to_keys.remove(&address) {
                for key in keys {
                    Self::revoke_key(key);
                }
            }
        });

        Ok(())
    }

    pub fn revoke_key(key: String) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let usr = state.api_keys.keys_to_user.remove(&key);
            state
                .api_keys
                .user_to_keys
                .get_mut(&usr.unwrap().address)
                .unwrap()
                .remove(&key);
        });
    }

    pub fn update_request_limit(
        address: String,
        key: String,
        new_limit: u64,
    ) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(user) = state.api_keys.keys_to_user.get_mut(&key) {
                if user.address != address {
                    return Err(APIKeysError::InvalidOwner);
                }
                user.request_limit = new_limit;
            } else {
                return Err(APIKeysError::InvalidKey);
            }
            Ok(())
        })
    }

    pub fn update_request_limit_by_domain(
        address: String,
        key: String,
        new_limit: u64,
    ) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(user) = state.api_keys.keys_to_user.get_mut(&key) {
                if user.address != address {
                    return Err(APIKeysError::InvalidOwner);
                }
                user.request_limit_by_domain = new_limit;
            } else {
                return Err(APIKeysError::InvalidKey);
            }
            Ok(())
        })
    }

    pub fn update_free_request_limit(new_limit: u64) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.api_keys.free_request_limit = new_limit;
        });
    }

    pub fn get_api_keys() -> APIKeys {
        STATE.with(|state| {
            let state = state.borrow();
            state.api_keys.clone()
        })
    }

    pub fn ban_domain(address: String, key: String, domain: String) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(user) = state.api_keys.keys_to_user.get_mut(&key) {
                if user.address != address {
                    return Err(APIKeysError::InvalidOwner);
                }

                user.ban_domain(domain);
            } else {
                return Err(APIKeysError::InvalidKey);
            }
            Ok(())
        })
    }

    pub fn allow_domain(address: String, key: String, domain: String) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(user) = state.api_keys.keys_to_user.get_mut(&key) {
                if user.address != address {
                    return Err(APIKeysError::InvalidOwner);
                }

                user.allow_domain(domain);
            } else {
                return Err(APIKeysError::InvalidKey);
            }
            Ok(())
        })
    }

    pub fn change_public_status(
        address: String,
        key: String,
        is_public: bool,
    ) -> Result<(), APIKeysError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(user) = state.api_keys.keys_to_user.get_mut(&key) {
                if user.address != address {
                    return Err(APIKeysError::InvalidOwner);
                }

                user.is_public = is_public;
            } else {
                return Err(APIKeysError::InvalidKey);
            }
            Ok(())
        })
    }
}
