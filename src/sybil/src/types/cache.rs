use std::{collections::HashMap, time::Duration};
use std::future::Future;

use ic_cdk::{
    export::{
        candid::CandidType,
        serde::{Deserialize, Serialize},
    },
};

use anyhow::Result;

use crate::STATE;

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
pub struct Cache {
    records: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Default, CandidType, Serialize, Deserialize)]
struct CacheEntry {
    expired_at: u64,
    data: Vec<u8>,
}

impl Cache {
    pub fn add_entry(&mut self, key: String, data: Vec<u8>) {
        let expiration_time = STATE.with(|state| state.borrow().cache_expiration);

        let entry = CacheEntry {
            expired_at: Duration::from_nanos(ic_cdk::api::time()).as_secs() + expiration_time,
            data,
        };

        self.records.insert(key, entry);
    }

    pub fn get_entry(&mut self, key: &str) -> Option<Vec<u8>> {
        let entry = self.records.get(key);

        if let Some(entry) = entry {
            if entry.expired_at < Duration::from_nanos(ic_cdk::api::time()).as_secs() {
                self.records.remove(key);
                return None;
            }

            return Some(entry.data.clone());
        }

        None
    }

    pub async fn execute<'a, T, F, Fmt, Arg>(&mut self, key: &str, f: F, arg: Arg) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + 'a,
        F: FnOnce(Arg) -> Fmt,
        Fmt: Future<Output = Result<T>>,
    {
        let entry = self.get_entry(key);

        if let Some(entry) = entry {
            return Ok(serde_json::from_slice(&entry)?);
        }
        let result = f(arg).await?;
        let serialized = serde_json::to_vec(&result)?;
        self.add_entry(key.to_string(), serialized);

        Ok(result)
    }
}
