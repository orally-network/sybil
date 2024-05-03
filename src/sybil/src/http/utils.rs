use crate::{
    log,
    types::{allowances::Allowances, api_keys::APIKeys, http::HttpRequest},
    HTTP_REQUESTS_COUNTER,
};

use anyhow::Result;

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
        .find(|(k, _)| k == "referer" || k == "origin")
        .map(|(_, v)| v)
        .cloned()
    else {
        return Ok((None, false));
    };

    let requests_per_domain = HTTP_REQUESTS_COUNTER.with(|c| {
        let mut c = c.borrow_mut();
        let counter = c.entry(domain.clone()).or_insert(0);
        *counter += 1;

        *counter - 1
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
