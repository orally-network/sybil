use crate::log;
use crate::types::chains_rpc::{TransactionRequest, EthCallArgs, BlockTag};

/// Default JSON response bytes length, result excluded
/// E.g. {"jsonrpc":"2.0","id":100,"result":""} is 37 bytes
pub const DEFAULT_JSON_RESPONSE_BYTES_LEN: u64 = 38;

/// Default JSON response bytes length, result excluded, with additional byte for comma ','
pub const DEFAULT_JSON_BATCH_RESPONSE_BYTES_LEN: u64 = DEFAULT_JSON_RESPONSE_BYTES_LEN + 1;

/// Additional bytes length for JSON batch response for '[' and ']'
pub const JSON_BATCH_ADDITIONAL_BYTES_LEN: u64 = 2;

/// result field length in JSON response for get block request
/// E.g. {"jsonrpc":"2.0","id":1,"result":"0x13c0a19"}, result here is 9 bytes, excluding quotes
pub const DEFAULT_GET_BLOCK_RESPONSE_LEN: u64 = 9;

/// Default U256 response length
/// E.g. 0x0000000000000000000000000000000000000000000000000000000000000012 is 66 bytes
pub const DEFAULT_U256_RESPONSE_LEN: u64 = 66;

/// Default string response length for the strings up to 32 bytes
pub const DEFAULT_STRING_RESPONSE_LEN: u64 = 194;

pub const DEFAULT_GET_RESERVES_RESPONSE_LEN: u64 = 194;

/// Approximate length of headers in bytes
pub const APPROXIMATE_HEADERS_LEN: u64 = 700;



pub fn get_block_response_len() -> u64 {
    let tmp = DEFAULT_JSON_RESPONSE_BYTES_LEN + DEFAULT_GET_BLOCK_RESPONSE_LEN + APPROXIMATE_HEADERS_LEN;
    log!("get_get_block_response_len: {}", tmp);
    tmp
}

pub fn get_token_address_batch_response_len<T: Into<u64>>(batch_size: T) -> u64 {
    let len = batch_size.into() * 2;

    APPROXIMATE_HEADERS_LEN + 
    DEFAULT_JSON_BATCH_RESPONSE_BYTES_LEN * len // Responses without 'result' field
        + JSON_BATCH_ADDITIONAL_BYTES_LEN // '[' and ']'
        + DEFAULT_U256_RESPONSE_LEN * len // 'result' field
}

pub fn get_decimals_and_symbols_batch_response_len<T: Into<u64>>(batch_size: T) -> u64 {
    let len = batch_size.into() * 2;

    let tmp = APPROXIMATE_HEADERS_LEN + 
    DEFAULT_JSON_BATCH_RESPONSE_BYTES_LEN * 2 * len // Responses without 'result' field
        + JSON_BATCH_ADDITIONAL_BYTES_LEN // '[' and ']'
        + (DEFAULT_STRING_RESPONSE_LEN + DEFAULT_U256_RESPONSE_LEN) * len; // 'result' fields
    
    log!("get_deciamls_and_symbols_batch_response_len: {}", tmp);

    tmp
}

pub fn get_reserves_batch_response_len<T: Into<u64>>(batch_size: T) -> u64 {
    let len = batch_size.into();

    APPROXIMATE_HEADERS_LEN + 
    DEFAULT_JSON_BATCH_RESPONSE_BYTES_LEN * 4 * len // Responses without 'result' field
        + JSON_BATCH_ADDITIONAL_BYTES_LEN // '[' and ']'
        + (DEFAULT_GET_RESERVES_RESPONSE_LEN + DEFAULT_U256_RESPONSE_LEN) * 2 * len // 'result' fields
}

// convert json_payload to eth_call args struct for correct method invocation
pub fn convert_to_call_args(arr: &[serde_json::Value]) -> Result<EthCallArgs, ic_web3_rs::Error> {
    if arr.len() < 2 {
        return Err(ic_web3_rs::Error::InvalidResponse(
            "Invalid arguments for eth_call: not enough elements".to_string(),
        ));
    }

    let obj = arr.get(0).and_then(|v| v.as_object()).ok_or_else(|| {
        ic_web3_rs::Error::InvalidResponse(format!("batch response is missing id"))
    })?;

    let input = obj
        .get("data")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            ic_web3_rs::Error::InvalidResponse("Missing or invalid 'data' in JSON object".to_string())
        })?;

    let from = obj
        .get("from")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            ic_web3_rs::Error::InvalidResponse("Missing or invalid 'from' in JSON object".to_string())
        })?;

    let to = obj
        .get("to")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            ic_web3_rs::Error::InvalidResponse("Missing or invalid 'to' in JSON object".to_string())
        })?;

    let transaction = TransactionRequest {
        input: Some(input),
        from: Some(from),
        to: Some(to),
        ..Default::default()
    };

    Ok(EthCallArgs {
        transaction,
        block: Some(BlockTag::Latest),
    })
}