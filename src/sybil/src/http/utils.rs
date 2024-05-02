use crate::types::{allowances::Allowances, api_keys::APIKeys, http::HttpRequest};

use anyhow::Result;

pub async fn resolve_payer(
    req: &HttpRequest,
    method: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
) -> Result<Option<String>> {
    let domain = req
        .headers
        .iter()
        .find(|(k, _)| k == "referer" || k == "origin")
        .map(|(_, v)| v)
        .cloned();

    let caller = match (msg, sig, api_key) {
        (Some(msg), Some(sig), _) => crate::utils::siwe::recover(&msg, &sig).await?,
        (_, _, Some(api_key)) => {
            let (address, is_free) = APIKeys::auth_key(api_key, method, domain.clone())?;

            if is_free {
                return Ok(None);
            }

            address
        }
        _ => ic_cdk::caller().to_string(),
    };

    let payer = check_for_potential_grantee(domain)?.unwrap_or(caller);

    Ok(Some(payer))
}

fn check_for_potential_grantee(grantee: Option<String>) -> Result<Option<String>> {
    if let Some(grantee) = grantee {
        Ok(Allowances::get_allowed_user(&grantee)?)
    } else {
        Ok(None)
    }
}
