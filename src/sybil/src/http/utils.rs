use crate::{
    log,
    types::{
        allowances::Allowances,
        api_keys::APIKeys,
        http::{APIRequest, HttpRequest},
    },
    utils::time::in_seconds,
    HTTP_REQUESTS,
};

use anyhow::Result;
use time_rs::OffsetDateTime;

pub fn get_domain(req: &HttpRequest) -> Option<String> {
    log!("HEADERS: {:?}", req.headers);
    if let Some(origin) = req
        .headers
        .iter()
        .find(|(k, _)| k == "origin")
        .map(|(_, v)| v)
        .cloned()
    {
        return Some(origin);
    }

    if let Some(referer) = req
        .headers
        .iter()
        .find(|(k, _)| k == "referer")
        .map(|(_, v)| v)
        .cloned()
    {
        return Some(referer);
    }

    if let Some(x_real_ip) = req
        .headers
        .iter()
        .find(|(k, _)| k == "x-real-ip")
        .map(|(_, v)| v)
        .cloned()
    {
        return Some(x_real_ip);
    }

    if let Some(x_forwarded_for) = req
        .headers
        .iter()
        .find(|(k, _)| k == "x-forwarded-for")
        .map(|(_, v)| v)
        .cloned()
    {
        return Some(x_forwarded_for);
    }

    None
}

pub async fn resolve_payer(
    domain: Option<String>,
    method: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
) -> Result<(Option<String>, bool)> {
    let Some(domain) = domain else {
        return Ok((None, false));
    };

    let _requests_per_domain = HTTP_REQUESTS.with(|c| {
        let mut c = c.borrow_mut();
        let api_request = c.entry(domain.clone()).or_insert(APIRequest {
            count: 0,
            method: method.clone(),
            last_request: in_seconds(),
        });

        api_request.count += 1;
        api_request.method = method.clone();

        let now = OffsetDateTime::from_unix_timestamp(in_seconds() as i64)
            .unwrap()
            .date();

        let previous_date = OffsetDateTime::from_unix_timestamp(api_request.last_request as i64)
            .unwrap_or_else(|_| {
                OffsetDateTime::from_unix_timestamp_nanos(api_request.last_request as i128).unwrap()
            })
            .date();

        if now != previous_date {
            api_request.count = 1;
        }

        api_request.last_request = in_seconds();

        api_request.count - 1
    });

    match (msg, sig, api_key) {
        (Some(msg), Some(sig), _) => {
            let payer = crate::utils::siwe::recover(&msg, &sig).await?;
            Ok((Some(payer), false))
        }
        (_, _, Some(api_key)) => {
            let (address, is_free) = APIKeys::auth_key(api_key, method, Some(domain.clone()))?;
            log!("API KEY: {:?}", address);
            log!("IS FREE: {:?}", is_free);

            Ok((Some(address), is_free))
        }
        _ => {
            let payer = Allowances::get_allowed_user(&domain);

            if payer.is_some() {
                Allowances::update(domain, method.clone())
                    .expect("update fails only if domain is not exist, but here it is exist");

                Ok((payer, false))
            } else {
                Ok((None, false))
            }
        }
    }
}
