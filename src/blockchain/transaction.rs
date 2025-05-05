use std::{
    fmt::{Display, Formatter, Result},
    time::{SystemTime, UNIX_EPOCH},
};

use hex::encode;
use rand::{Rng, rng};
use ring::digest::{Digest, SHA256, digest};
use serde::{Deserialize, Serialize};
use serde_json::to_vec;

/// Represents a transaction in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub timestamp_ms: u64,
    pub signature: String, // In a real implementation, this would be a cryptographic signature.
}

impl Transaction {
    pub fn new(sender: &str, recipient: &str, amount: u64) -> Self {
        let now: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        // Generate a random transaction ID (in a real implementation, this would be more secure).
        let id: String = format!("tx-{}-{}", now, rng().random::<u32>());
        // In a real implementation, this would be a cryptographic signature.
        let signature: String = {
            let message: String = format!("{}{}{}{}", sender, recipient, amount, now);
            let hash_result: Digest = digest(&SHA256, message.as_bytes());
            encode(hash_result.as_ref())
        };

        return Self {
            id,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            amount,
            timestamp_ms: now,
            signature,
        };
    }

    /// Verify the transaction signature.
    pub fn verify(&self) -> bool {
        // In a real implementation, this would verify the cryptographic signature.
        let message: String = format!("{}{}{}{}", self.sender, self.recipient, self.amount, self.timestamp_ms);
        let hash_result: Digest = digest(&SHA256, message.as_bytes());
        let calculated_signature: String = encode(hash_result.as_ref());

        return calculated_signature == self.signature;
    }

    /// Convert transaction to bytes for PoH event insertion.
    pub fn to_bytes(&self) -> Vec<u8> {
        return to_vec(self).unwrap_or_default();
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        return write!(f, "Transaction[{}]: {} → {} ({} units)", self.id, self.sender, self.recipient, self.amount);
    }
}
