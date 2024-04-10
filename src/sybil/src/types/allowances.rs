use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::STATE;

// Allowances is a map that contains which domains (grantee) are allowed to use which user's (grantor) balance
// grantee domain => grantor public key
#[derive(Default, Serialize, Deserialize, CandidType, Debug, Clone)]
pub struct Allowances(HashMap<String, String>);

impl Allowances {
    pub fn grant(domain: String, grantor: String) -> Result<(), url::ParseError> {
        let url = url::Url::parse(&domain)?;
        let grantee = url.host_str().unwrap().to_string();

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let inner = state.allowances.0.borrow_mut();

            if inner.get(&grantee).is_none() {
                inner.insert(grantee, grantor);
            }
        });

        Ok(())
    }

    pub fn restrict(domain: String, grantor: String) -> Result<(), url::ParseError> {
        let url = url::Url::parse(&domain)?;
        let grantee = url.host_str().unwrap().to_string();
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let inner = state.allowances.0.borrow_mut();

            if inner.get(&grantee) == Some(&grantor) {
                inner.remove(&grantee);
            }
        });

        Ok(())
    }

    pub fn get_allowed_user(domain: &str) -> Result<Option<String>, url::ParseError> {
        let url = url::Url::parse(domain)?;
        let grantee = url.host_str().unwrap().to_string();

        STATE.with(|state| {
            let state = state.borrow();
            let inner = state.allowances.0.borrow();

            Ok(inner.get(&grantee).cloned())
        })
    }
}
