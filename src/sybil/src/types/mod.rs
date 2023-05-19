pub mod cache;
pub mod custom_pair;
pub mod rate_data;
pub mod state;
pub mod http;

pub enum PairType {
    CustomPair,
    Pair,
}
