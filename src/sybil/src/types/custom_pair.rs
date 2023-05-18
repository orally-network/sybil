use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};

use anyhow::{anyhow, Result};
use url::Url;

use super::rate_data::RateDataLight;
use crate::utils::{get_rate::get_custom_rate_with_cache, is_valid_pair_id};

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

        Ok(Self {
            id: builder.id.clone(),
            frequency,
            source,
            data,
            last_update: ic_cdk::api::time(),
        })
    }
}

pub struct CustomPairBuilder {
    id: String,
    frequency: Option<u64>,
    source: Option<Endpoint>,
    data: Option<RateDataLight>,
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

        let (rate, expected_bytes) = get_custom_rate_with_cache(&url, resolver, &self.id).await?;

        self.source = Some(Endpoint {
            uri: uri.to_string(),
            resolver: resolver.into(),
            expected_bytes,
        });
        self.data = Some(rate);

        Ok(self)
    }

    pub fn build(self) -> Result<CustomPair> {
        CustomPair::new(&self)
    }
}
