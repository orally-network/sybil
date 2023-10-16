pub mod balances;
pub mod cache;
pub mod config;
pub mod data_fetchers;
pub mod exchange_rate;
pub mod http;
pub mod pairs;
pub mod rate_data;
pub mod state;
pub mod whitelist;
pub mod auth_data;
pub mod service;

pub type Timestamp = u64;
pub type Seconds = u64;
pub type Address = String;
