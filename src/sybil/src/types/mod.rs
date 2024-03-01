pub mod balances;
pub mod cache;
pub mod config;
pub mod exchange_rate;
pub mod feeds;
pub mod http;
pub mod pagination;
pub mod rate_data;
pub mod source;
pub mod state;
pub mod whitelist;

pub type Timestamp = u64;
pub type Seconds = u64;
pub type Address = String;

pub const SUPER_MSG: &str = "investly";
pub const SUPER_SIG: &str = "signed investly";
pub const SUPER_USER: &str = "0x654DFF41D51c230FA400205A633101C5C1f1969C";
