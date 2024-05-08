pub mod get_asset_data;
pub mod get_multiple_asset_data;
pub mod get_xrc_data;
pub mod read_contract;
pub mod read_logs;

use super::{response, HttpResponse};

pub use get_asset_data::*;
pub use get_multiple_asset_data::*;
pub use get_xrc_data::*;
pub use read_contract::*;
pub use read_logs::*;

pub async fn gather_metrics() -> HttpResponse {
    let data = crate::utils::metrics::gather_metrics();

    response::ok(data)
}
