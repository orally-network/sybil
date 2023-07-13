use anyhow::Result;
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{response, HttpRequest, HttpResponse};
use crate::{types::pairs::PairsStorage, utils::validation};

#[derive(Debug, PartialEq, Deserialize, Serialize, Validate)]
struct GetAssetDataQueryParams {
    #[validate(regex = "validation::PAIR_ID_REGEX")]
    pair_id: String,
    signature: Option<bool>,
}

impl TryFrom<String> for GetAssetDataQueryParams {
    type Error = serde_qs::Error;

    fn try_from(query: String) -> Result<Self, serde_qs::Error> {
        serde_qs::from_str(&query)
    }
}

pub async fn get_asset_data_request(req: HttpRequest) -> HttpResponse {
    let resp = _get_asset_data_request(req)
        .await
        .map_err(|e| e.to_string());

    match resp {
        Ok(data) => response::ok(data),
        Err(err) => response::bad_request(err),
    }
}

#[inline(always)]
async fn _get_asset_data_request(req: HttpRequest) -> Result<Vec<u8>> {
    let params = GetAssetDataQueryParams::try_from(req.url)?;
    params.validate()?;

    let rate = PairsStorage::rate(&params.pair_id, params.signature.unwrap_or(false)).await?;

    Ok(serde_json::to_vec(&rate)?)
}
