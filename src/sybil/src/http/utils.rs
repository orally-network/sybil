use crate::{
    types::{
        allowances::Allowances,
        api_keys::APIKeys,
        http::{APIRequest, HttpRequest},
    },
    HTTP_REQUESTS,
};

use anyhow::Result;
use ic_cdk::api::time;
use time_rs::OffsetDateTime;

const MAX_FREE_REQUESTS: u128 = 100;

pub async fn resolve_payer(
    req: &HttpRequest,
    method: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
) -> Result<(Option<String>, bool)> {
    let Some(domain) = req
        .headers
        .iter()
        .find(|(k, _)| {
            k == "referer" || k == "origin" || k == "x-real-ip" || k == "x-forwarded-for"
        })
        .map(|(_, v)| v)
        .cloned()
    else {
        return Ok((None, false));
    };

    let requests_per_domain = HTTP_REQUESTS.with(|c| {
        let mut c = c.borrow_mut();
        let api_request = c.entry(domain.clone()).or_insert(APIRequest {
            count: 0,
            method: method.clone(),
            last_request: time(),
        });

        api_request.count += 1;
        api_request.method = method.clone();

        let now = OffsetDateTime::from_unix_timestamp_nanos(ic_cdk::api::time() as i128)
            .unwrap()
            .date();

        let previous_date =
            OffsetDateTime::from_unix_timestamp_nanos(api_request.last_request as i128)
                .unwrap()
                .date();

        if now != previous_date {
            api_request.count = 1;
        }

        api_request.last_request = time();

        api_request.count - 1
    });

    if requests_per_domain < MAX_FREE_REQUESTS {
        return Ok((None, true));
    }

    let caller = match (msg, sig, api_key) {
        (Some(msg), Some(sig), _) => crate::utils::siwe::recover(&msg, &sig).await?,
        (_, _, Some(api_key)) => {
            let (address, is_free) = APIKeys::auth_key(api_key, method, Some(domain.clone()))?;

            return Ok((Some(address), is_free));
        }
        _ => return Ok((None, false)),
    };

    let payer = Allowances::get_allowed_user(&domain)?.unwrap_or(caller);

    Ok((Some(payer), false))
}
