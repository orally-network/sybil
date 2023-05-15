use anyhow::Result;
use url::Url;

use ic_utils::logger::log_message;
use ic_web3::types::H160;

use super::get_rate_with_cache;
use crate::{types::custom_pair::Endpoint, STATE};

pub fn update_rate(source: Endpoint, pair_id: String, pub_key: H160) {
    ic_cdk::spawn(_update_rate(source, pair_id, pub_key));
}

async fn _update_rate(source: Endpoint, pair_id: String, pub_key: H160) {
    let result = __update_rate(source, pair_id, pub_key).await;

    if let Err(err) = result {
        log_message(format!("[{pub_key}] {err}"))
    }
}

async fn __update_rate(source: Endpoint, pair_id: String, pub_key: H160) -> Result<()> {
    let url = Url::parse(&source.uri)?;

    let (rate, _) = get_rate_with_cache(&url).await?;

    rate.verify(&pub_key)?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        let mut custom_pair = state
            .custom_pairs
            .iter_mut()
            .find(|p| p.id == pair_id)
            .expect("custom pair should exists");

        custom_pair.data = rate
    });

    Ok(())
}
