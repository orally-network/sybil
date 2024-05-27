use std::collections::{HashMap, HashSet};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::utils::siwe::SiweError;
use crate::utils::time::in_seconds;
use crate::utils::{address, CallerError};
use crate::STATE;

#[derive(Error, Debug)]
pub enum AllowancesError {
    #[error("SIWE error: {0}")]
    SiweError(#[from] SiweError),
    #[error("Address error: {0}")]
    AddressError(#[from] address::AddressError),
    #[error("Caller error: {0}")]
    CallerError(#[from] CallerError),
    #[error("Invalid api domain")]
    InvalidKey,
}

#[derive(Default, Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct Allowance {
    pub grantor_address: String,
    pub request_count: u64,
    pub request_count_per_method: HashMap<String, u64>,
    pub request_count_per_domain: HashMap<String, u64>,
    pub last_request: u64, // timestamp of the last request
}

impl Allowance {
    pub fn increment_request_count_by_method(&mut self, method: &str) {
        self.request_count += 1;
        *self
            .request_count_per_method
            .entry(method.to_string())
            .or_default() += 1;
        self.last_request = in_seconds();
    }

    pub fn increment_request_count_by_domain(&mut self, domain: &str) {
        self.request_count += 1;
        *self
            .request_count_per_domain
            .entry(domain.to_string())
            .or_default() += 1;
        self.last_request = in_seconds();
    }
}

// Allowances is a map that contains which domains (grantee) are allowed to use which user's (grantor) balance
// grantee domain => grantor user
#[derive(Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct Allowances {
    domains_to_allowances: HashMap<String, Allowance>,
    user_to_allowed_domains: HashMap<String, HashSet<String>>,
}

impl Default for Allowances {
    fn default() -> Self {
        Self {
            domains_to_allowances: HashMap::new(),
            user_to_allowed_domains: HashMap::new(),
        }
    }
}

impl Allowances {
    // Updates info about allowance
    pub fn update(domain: String, method: String) -> Result<(), AllowancesError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            let Some(allowance) = state.allowances.domains_to_allowances.get_mut(&domain) else {
                return Err(AllowancesError::InvalidKey);
            };

            allowance.increment_request_count_by_method(&method);
            allowance.increment_request_count_by_domain(&domain);
            allowance.last_request = in_seconds();

            Ok(())
        })
    }

    pub fn get_allowed_user(domain: &str) -> Option<String> {
        STATE.with(|state| {
            let state = state.borrow();
            state
                .allowances
                .domains_to_allowances
                .get(domain)
                .map(|a| a.grantor_address.clone())
        })
    }

    pub fn get_user_allowed_domains(address: &str) -> Result<Vec<String>, AllowancesError> {
        let address = address::from_str(address)?;

        Ok(STATE.with(|state| {
            let mut state = state.borrow_mut();
            state
                .allowances
                .user_to_allowed_domains
                .entry(address.clone())
                .or_default()
                .clone()
                .into_iter()
                .collect()
        }))
    }

    pub fn get_allowance_by_domain(domain: &str) -> Option<Allowance> {
        STATE.with(|state| {
            let state = state.borrow();
            state.allowances.domains_to_allowances.get(domain).cloned()
        })
    }

    pub fn restrict_domains_by_user(address: &str) -> Result<(), AllowancesError> {
        let address = address::from_str(address)?;

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(domains) = state.allowances.user_to_allowed_domains.remove(&address) {
                for domain in domains {
                    Self::restrict(domain);
                }
            }
        });

        Ok(())
    }

    pub fn restrict(domain: String) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let allowance = state.allowances.domains_to_allowances.remove(&domain);
            state
                .allowances
                .user_to_allowed_domains
                .get_mut(&allowance.unwrap().grantor_address)
                .unwrap()
                .remove(&domain);
        });
    }

    pub fn grant(domain: String, grantor: String) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let old_allowance = state.allowances.domains_to_allowances.insert(
                domain.clone(),
                Allowance {
                    grantor_address: grantor.clone(),
                    ..Default::default()
                },
            );

            if let Some(old_allowance) = old_allowance {
                state
                    .allowances
                    .user_to_allowed_domains
                    .get_mut(&old_allowance.grantor_address)
                    .unwrap()
                    .remove(&domain);
            }

            state
                .allowances
                .user_to_allowed_domains
                .entry(grantor)
                .or_default()
                .insert(domain);
        });
    }
}
