use std::collections::HashMap;
use std::future::Future;

use ic_cdk::{
    api::time,
    export::serde::{Deserialize, Serialize},
};

use anyhow::Result;

const EXPIRATION_TIME: u64 = 60 * 60; // 1 hour

#[derive(Debug, Clone, Default)]
pub struct Cache {
    records: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Default)]
struct CacheEntry {
    expired_at: u64,
    data: Vec<u8>,
}

impl Cache {
    pub fn add_entry(&mut self, key: String, data: Vec<u8>) {
        let entry = CacheEntry {
            expired_at: time() + EXPIRATION_TIME,
            data,
        };

        self.records.insert(key, entry);
    }

    pub fn get_entry(&mut self, key: &str) -> Option<Vec<u8>> {
        let entry = self.records.get(key);

        if let Some(entry) = entry {
            if entry.expired_at < time() {
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
