use std::time::Instant;

use lib::utils::serialization;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoHRecord {
    pub tick_index: u64,
    pub slot_index: u64,
    pub epoch_index: u64,
    #[serde(with = "serialization")]
    pub hash: [u8; 32],
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<u8>>,
}

pub struct PoH {
    pub current_hash: [u8; 32],
    pub tick_count: u64,
    pub slot_count: u64,
    pub epoch_count: u64,
    pub start_time: Instant,
}
