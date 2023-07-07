pub mod cache;
pub mod config;
pub mod custom_pair;
pub mod http;
pub mod rate_data;
pub mod state;
pub mod pairs;
pub mod balances;

#[derive(Clone, Debug)]
pub enum PairType {
    CustomPair,
    Pair,
}

pub type Timestamp = u64;
pub type Seconds = u64;
pub type Address = String;
