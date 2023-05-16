mod custom_pairs;
mod pairs;
mod state;

use ic_cdk::query;
use crate::utils::is_pair_exist;

#[query]
fn is_pair_exists(pair_id: String) -> bool {
    is_pair_exist(&pair_id)
}
