use std::str::FromStr;

use crate::utils::{validation, web3};
use candid::CandidType;
use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpHeader};
use ic_web3_rs::{
    ethabi::{Contract, RawLog, Token},
    types::{H160, H256},
};
use jsonptr::{Pointer, Resolve};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::{
    clone_with_state, defer, jobs::cache_cleaner, retry_until_success, types::cache::HttpCache,
};

use super::{cache::HttpCacheError, feeds::RateResult, Seconds};

const ORALLY_WRAPPER_CAHCHE_TTL: u64 = 30000; // 30 seconds
const MIN_EXPECTED_BYTES: u64 = 1;
const MAX_EXPECTED_BYTES: u64 = 1024 * 1024 * 2;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct ApiKey {
    pub title: String,
    pub key: String,
}

impl ApiKey {
    pub fn censor(&mut self) {
        self.key = "***".to_string();
    }
}

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("Failed to get logs: {0}")]
    FailedToGetLogs(String),
    #[error("Failed to parse logs: {0}")]
    FailedToParseLogs(String),
    #[error("Failed to parse abi: {0}")]
    FailedToParseABI(String),
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Log field name doesn't exist in log params")]
    LogFieldNotFound(String),
    #[error("HttpCache error: {0}")]
    HttpCacheError(#[from] HttpCacheError),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Web3 error: {0}")]
    Web3Error(#[from] web3::Web3Error),
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum Source {
    HttpSource(HttpSource),
    EvmEventLogsSource(EvmEventLogsSource),
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct EvmEventLogsSource {
    #[validate(url)]
    pub rpc: String,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub address: Option<String>,
    pub topic: Option<String>,
    pub block_hash: Option<String>,
    pub log_index: u32,
    pub event_log_field_name: String,
    pub event_name: String,
    pub event_abi: String,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize, Validate)]
pub struct HttpSource {
    #[validate(url)]
    pub uri: String,
    pub api_keys: Option<Vec<ApiKey>>,
    #[validate(regex = "validation::RATE_RESOLVER")]
    pub resolver: String,
    #[validate(range(min = "MIN_EXPECTED_BYTES", max = "MAX_EXPECTED_BYTES"))]
    pub expected_bytes: Option<u64>,
}

impl HttpSource {
    pub fn get_url_with_keys(&self) -> String {
        let mut url = self.uri.clone();

        if let Some(api_keys) = &self.api_keys {
            for api_key in api_keys {
                url = url.replace(&format!("{{{}}}", &api_key.title), &api_key.key);
            }
        }

        url
    }
}

impl Source {
    pub async fn rate(&self, expr_freq: Seconds) -> Result<RateResult, SourceError> {
        match self {
            Source::HttpSource(http_source) => Source::http_rate(http_source, expr_freq).await,
            Source::EvmEventLogsSource(evm_event_logs_source) => {
                Source::evm_event_logs_rate(evm_event_logs_source, expr_freq).await
            }
        }
    }

    pub async fn evm_event_logs_rate(
        evm_event_logs_source: &EvmEventLogsSource,
        _expr_freq: Seconds,
    ) -> Result<RateResult, SourceError> {
        evm_event_logs_source.validate()?;

        let rpc_wrapper = clone_with_state!(rpc_wrapper);
        let url = format!(
            "{}{}&cacheTTL={}",
            rpc_wrapper,
            urlencoding::encode(&evm_event_logs_source.rpc),
            ORALLY_WRAPPER_CAHCHE_TTL
        );

        let w3 = web3::instance(url, clone_with_state!(evm_rpc_canister));

        let topic = if let Some(topic) = &evm_event_logs_source.topic {
            Some(
                H256::from_str(&topic)
                    .map_err(|err| SourceError::InvalidRequest(err.to_string()))?,
            )
        } else {
            None
        };

        let address = if let Some(address) = &evm_event_logs_source.address {
            Some(
                H160::from_str(&address)
                    .map_err(|err| SourceError::InvalidRequest(err.to_string()))?,
            )
        } else {
            None
        };

        let block_hash = if let Some(block_hash) = &evm_event_logs_source.block_hash {
            Some(
                H256::from_str(&block_hash)
                    .map_err(|err| SourceError::InvalidRequest(err.to_string()))?,
            )
        } else {
            None
        };

        let mut logs = w3
            .get_logs(
                evm_event_logs_source.from_block,
                evm_event_logs_source.to_block,
                topic,
                address,
                block_hash,
            )
            .await
            .map_err(|err| SourceError::FailedToGetLogs(err.to_string()))?;

        let contract = Contract::load(evm_event_logs_source.event_abi.as_bytes())
            .map_err(|err| SourceError::FailedToParseABI(err.to_string()))?;

        let event = contract
            .event(&evm_event_logs_source.event_name)
            .map_err(|err| SourceError::FailedToParseABI(err.to_string()))?;

        if logs.len() <= evm_event_logs_source.log_index as usize {
            return Err(SourceError::InvalidRequest(
                "Log index is out of range".to_string(),
            ));
        }

        // Caution: log_index is used to move out the log from the logs vector
        // logs vector will be changed after the next line
        let log_at_index = logs.swap_remove(evm_event_logs_source.log_index as usize);

        let raw_log = RawLog {
            topics: log_at_index.topics,
            data: log_at_index.data.0,
        };

        let log = event
            .parse_log(raw_log)
            .map_err(|err| SourceError::FailedToParseLogs(err.to_string()))?;

        let token = log
            .params
            .into_iter()
            .find(|p| p.name == evm_event_logs_source.event_log_field_name)
            .ok_or(SourceError::LogFieldNotFound(
                evm_event_logs_source.event_log_field_name.clone(),
            ))?
            .value;

        let data = match token {
            Token::Int(val) | Token::Uint(val) => serde_json::to_value(val.to_string())?,
            Token::FixedArray(val) | Token::Array(val) | Token::Tuple(val) => {
                serde_json::to_value(val)?
            }
            Token::String(val) => serde_json::to_value(val)?,
            Token::Bytes(val) | Token::FixedBytes(val) => serde_json::to_value(val)?,
            Token::Address(val) => serde_json::to_value(val)?,
            Token::Bool(val) => serde_json::to_value(val)?,
        };

        Ok(RateResult {
            rate: serde_json::to_value(&data)?,
            cached_at: 0,
            bytes: 0,
        })
    }

    pub async fn http_rate(
        http_source: &HttpSource,
        expr_freq: Seconds,
    ) -> Result<RateResult, SourceError> {
        http_source.validate()?;

        let rpc_wrapper = clone_with_state!(rpc_wrapper);
        let req = CanisterHttpRequestArgument {
            url: format!(
                "{}{}&cacheTTL={}",
                rpc_wrapper,
                urlencoding::encode(&http_source.get_url_with_keys()),
                ORALLY_WRAPPER_CAHCHE_TTL
            ),
            max_response_bytes: http_source.expected_bytes,
            headers: Self::get_default_headers(),
            ..Default::default()
        };

        defer!(cache_cleaner::execute());

        let (response, cached_at) =
            retry_until_success!(HttpCache::request_with_access(&req, expr_freq))?;
        let bytes = response.body.len();

        let ptr = Pointer::try_from(http_source.resolver.clone())
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?;

        let data = serde_json::from_slice::<Value>(&response.body)?;

        let rate = data
            .resolve(&ptr)
            .map_err(|err| HttpCacheError::InvalidResponseBodyResolver(format!("{err:?}")))?
            .clone();

        Ok(RateResult {
            rate,
            cached_at,
            bytes,
        })
    }

    pub fn search(&self, search: &str) -> bool {
        match self {
            Source::HttpSource(http_source) => {
                let uri = http_source.get_url_with_keys().trim().to_lowercase();
                strsim::jaro(&uri, search) >= 0.65
            }
            Source::EvmEventLogsSource(evm_event_logs_source) => {
                let rpc = evm_event_logs_source.rpc.trim().to_lowercase();
                let address = evm_event_logs_source
                    .address
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();
                let topic = evm_event_logs_source
                    .topic
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();
                let block_hash = evm_event_logs_source
                    .block_hash
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();

                let event_log_field_name = evm_event_logs_source
                    .event_log_field_name
                    .trim()
                    .to_lowercase();

                strsim::jaro(&rpc, search) >= 0.65
                    || strsim::jaro(&address, search) >= 0.65
                    || strsim::jaro(&topic, search) >= 0.65
                    || strsim::jaro(&block_hash, search) >= 0.65
                    || strsim::jaro(&event_log_field_name, search) >= 0.65
            }
        }
    }

    pub fn get_default_headers() -> Vec<HttpHeader> {
        vec![
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
            HttpHeader {
                name: "User-Agent".to_string(),
                value: "sybil".to_string(),
            },
        ]
    }
}
