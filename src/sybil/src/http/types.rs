use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetAssetDataQueryParams {
    pub id: String,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
}

impl TryFrom<String> for GetAssetDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetMultipleAssetsDataQueryParams {
    pub ids: String,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
}

impl TryFrom<String> for GetMultipleAssetsDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct GetXRCDataQueryParams {
    pub id: String,
    pub api_key: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub bytes: Option<bool>,
}

impl TryFrom<String> for GetXRCDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct ReadLogsQueryParams {
    pub chain_id: u64,
    pub block_from: Option<u64>,
    pub block_to: Option<u64>,
    pub topics0: Option<String>,
    pub topics1: Option<String>,
    pub topics2: Option<String>,
    pub topics3: Option<String>,
    pub addresses: Option<String>,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
}

impl TryFrom<String> for ReadLogsQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
pub struct ReadContractQueryParams {
    pub chain_id: u64,
    pub function_signature: String,
    pub contract_addr: String,
    pub method: String,
    pub params: String,
    pub msg: Option<String>,
    pub sig: Option<String>,
    pub api_key: Option<String>,
    pub bytes: Option<bool>,
}

impl TryFrom<String> for ReadContractQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}
