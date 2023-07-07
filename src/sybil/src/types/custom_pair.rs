use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use super::{Seconds, Timestamp};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Source {
    pub uri: String,
    pub resolver: String,
    pub expected_bytes: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CustomPair {
    pub id: String,
    pub update_freq: Seconds,
    pub source: Source,
    pub available_executions: u64,
    pub last_update: Timestamp,
}
