use crate::{
    log,
    types::api_keys::APIKeys,
};

use anyhow::Result;

pub async fn resolve_payer(
    method: String,
    msg: Option<String>,
    sig: Option<String>,
    api_key: Option<String>,
) -> Result<(Option<String>, bool)> {

    match (msg, sig, api_key) {
        (Some(msg), Some(sig), _) => {
            let payer = crate::utils::siwe::recover(&msg, &sig).await?;
            Ok((Some(payer), false))
        }
        (_, _, Some(api_key)) => {
            let (address, is_free) = APIKeys::auth_key(api_key, method, None)?;
            log!("API KEY: {:?}", address);
            log!("IS FREE: {:?}", is_free);

            Ok((Some(address), is_free))
        }
        _ => Ok((None, false)),
    }
}
