use std::time::Duration;

use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
};
use ic_cdk_timers::set_timer_interval;
use ic_web3::types::H160;

use anyhow::{anyhow, Result};
use url::Url;

use super::rate_data::CustomPairData;
use crate::utils::{get_rate_with_cache, is_pair_exist, update_rate::update_rate};

const MIN_FREQUENCY: u64 = 60;
const MAX_FREQUENCY: u64 = 24 * 60 * 60 * 365;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct Endpoint {
    pub uri: String,
    pub expected_bytes: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CustomPair {
    pub id: String,
    pub frequency: u64,
    pub source: Endpoint,
    pub data: CustomPairData,
    pub timer_id: String,
}

impl CustomPair {
    pub fn new(builder: &CustomPairBuilder) -> Result<Self> {
        let frequency = builder.frequency.ok_or(anyhow!("Frequency is required"))?;
        let source = builder
            .source
            .clone()
            .ok_or(anyhow!("Source is required"))?;
        let data = builder.data.clone().ok_or(anyhow!("Data is required"))?;

        let pub_key = builder.pub_key.ok_or(anyhow!("Public key is required"))?;
        let pair_id = builder.id.clone();
        let timer_source = source.clone();
        let timer_id = set_timer_interval(Duration::from_secs(frequency), move || {
            update_rate(timer_source.clone(), pair_id.clone(), pub_key);
        });

        Ok(Self {
            id: builder.id.clone(),
            frequency,
            source,
            data,
            timer_id: serde_json::to_string(&timer_id)?,
        })
    }
}

pub struct CustomPairBuilder {
    id: String,
    frequency: Option<u64>,
    source: Option<Endpoint>,
    data: Option<CustomPairData>,
    pub_key: Option<H160>,
}

impl CustomPairBuilder {
    pub fn new(pair_id: &str) -> Result<Self> {
        if is_pair_exist(pair_id) {
            return Err(anyhow::anyhow!("Pair already exists"));
        }

        Ok(Self {
            id: pair_id.to_string(),
            frequency: None,
            source: None,
            data: None,
            pub_key: None,
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

    pub async fn source(mut self, uri: &str, pub_key: &H160) -> Result<Self> {
        let url = Url::parse(uri)?;

        let (rate, expected_bytes) = get_rate_with_cache(&url).await?;

        rate.verify(pub_key)?;

        self.source = Some(Endpoint {
            uri: uri.to_string(),
            expected_bytes,
        });
        self.data = Some(rate);
        self.pub_key = Some(*pub_key);

        Ok(self)
    }

    pub fn build(self) -> Result<CustomPair> {
        CustomPair::new(&self)
    }
}
