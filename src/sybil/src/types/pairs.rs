use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use thiserror::Error;
use validator::Validate;

use super::{Timestamp, Seconds};
use crate::{
    methods::{
        custom_pairs::CreateCustomPairRequest,
        pairs::CreatePairRequest,
    },
    utils::{nat, validation},
};

const MIN_EXPECTED_BYTES: u64 = 1;
const MAX_EXPECTED_BYTES: u64 = 1024 * 1024 * 2;

#[derive(Error, Debug)]
pub enum PairError {
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct Source {
    #[validate(url)]
    pub uri: String,
    #[validate(regex = "validation::RATE_RESOLVER")]
    pub resolver: String,
    #[validate(range(min = "MIN_EXPECTED_BYTES", max = "MAX_EXPECTED_BYTES"))]
    pub expected_bytes: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub enum PairType {
    CustomPair {
        executions_left: u64,
        sources: Vec<Source>,
    },
    #[default]
    DefaultPair,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct PairStatus {
    last_update: Timestamp,
    updated_counter: u64,
    requests_counter: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Pair {
    pub id: String,
    pub pair_type: PairType,
    pub update_freq: Seconds,
    pub decimals: u64,
    pub status: PairStatus,
}

impl From<CreateCustomPairRequest> for Pair {
    fn from(req: CreateCustomPairRequest) -> Self {
        Self {
            id: req.pair_id,
            pair_type: PairType::CustomPair {
                executions_left: nat::to_u64(&req.executions),
                sources: req.endpoints,
            },
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}

impl From<CreatePairRequest> for Pair {
    fn from(req: CreatePairRequest) -> Self {
        Self {
            id: req.pair_id,
            pair_type: PairType::DefaultPair,
            update_freq: nat::to_u64(&req.update_freq),
            decimals: nat::to_u64(&req.decimals),
            ..Default::default()
        }
    }
}
