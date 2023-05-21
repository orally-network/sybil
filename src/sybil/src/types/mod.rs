pub mod cache;
pub mod custom_pair;
pub mod http;
pub mod rate_data;
pub mod state;

pub enum PairType {
    CustomPair,
    Pair,
}
