use std::time::Duration;

use candid::Nat;
use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use anyhow::{anyhow, Result};
use url::Url;

use super::rate_data::RateDataLight;
use crate::{
    utils::{
        get_rate::get_custom_rate_with_cache,
        is_valid_pair_id,
        treasurer::{deposit, DepositRequest, DepositType},
    },
    STATE,
};

const MIN_FREQUENCY: u64 = 60;
const MAX_FREQUENCY: u64 = 24 * 60 * 60 * 365;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Endpoint {
    pub uri: String,
    pub resolver: String,
    pub expected_bytes: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CustomPair {
    pub id: String,
    pub frequency: u64,
    pub source: Endpoint,
    pub data: RateDataLight,
    pub available_executions: u64,
    pub last_update: u64,
}

impl CustomPair {
    pub fn new(builder: &CustomPairBuilder) -> Result<Self> {
        let frequency = builder.frequency.ok_or(anyhow!("Frequency is required"))?;
        let source = builder
            .source
            .clone()
            .ok_or(anyhow!("Source is required"))?;
        let data = builder.data.clone().ok_or(anyhow!("Data is required"))?;
        let available_executions = builder
            .available_executions
            .ok_or(anyhow!("Avaible executions is required"))?;

        Ok(Self {
            id: builder.id.clone(),
            frequency,
            source,
            data,
            last_update: Duration::from_secs(ic_cdk::api::time()).as_secs(),
            available_executions,
        })
    }
}

pub struct CustomPairBuilder {
    id: String,
    frequency: Option<u64>,
    source: Option<Endpoint>,
    data: Option<RateDataLight>,
    available_executions: Option<u64>,
}

impl CustomPairBuilder {
    pub fn new(pair_id: &str) -> Result<Self> {
        if !is_valid_pair_id(pair_id) {
            return Err(anyhow!("Pair ID is invalid"));
        }

        Ok(Self {
            id: pair_id.to_string(),
            frequency: None,
            source: None,
            data: None,
            available_executions: None,
        })
    }

    pub fn frequency(mut self, frequency: u64) -> Result<Self> {
        if frequency < MIN_FREQUENCY {
            return Err(anyhow::anyhow!("Frequency must be at least 1 minute"));
        }

        if frequency > MAX_FREQUENCY {
            return Err(anyhow::anyhow!("Frequency must be less than a year"));
        }

        self.frequency = Some(frequency);

        Ok(self)
    }

    pub async fn source(mut self, uri: &str, resolver: &str) -> Result<Self> {
        let url = Url::parse(uri)?;

        let (rate, expected_bytes) = get_custom_rate_with_cache(&url, resolver, &self.id, true).await?;

        self.source = Some(Endpoint {
            uri: uri.to_string(),
            resolver: resolver.into(),
            expected_bytes,
        });
        self.data = Some(rate);

        Ok(self)
    }

    pub async fn estimate_cost(mut self, taxpayer: String, amount: Nat) -> Result<Self> {
        let cost_per_execution = STATE.with(|state| state.borrow().cost_per_execution);

        let available_executions = amount / cost_per_execution;

        let amount = available_executions.clone() * cost_per_execution;

        let req = DepositRequest {
            amount,
            taxpayer,
            deposit_type: DepositType::Erc20,
        };

        deposit(req).await.map_err(|e| anyhow!(e))?;

        let available_executions = *available_executions
            .0
            .to_u64_digits()
            .last()
            .expect("should contain u64");

        self.available_executions = Some(available_executions);

        Ok(self)
    }

    pub fn build(self) -> Result<CustomPair> {
        CustomPair::new(&self)
    }
}
