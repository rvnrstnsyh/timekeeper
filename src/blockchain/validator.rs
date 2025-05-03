use serde::{Deserialize, Serialize};

/// Represents a validator in the PoS blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: String,
    pub stake: u64,
    pub blocks_produced: u64,
    pub last_active_slot: u64,
    pub rewards_earned: u64,
}

impl Validator {
    /// Create a new validator with the specified address and stake.
    pub fn new(address: &str, stake: u64) -> Self {
        return Self {
            address: address.to_string(),
            stake,
            blocks_produced: 0,
            last_active_slot: 0,
            rewards_earned: 0,
        };
    }
}
