use std::fmt::{Display, Formatter, Result};

use crate::blockchain::transaction::Transaction;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Represents a block in the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub slot: u64,
    pub epoch: u64,
    pub timestamp_ms: u64,
    pub transactions: Vec<Transaction>,
    pub validator: String,
    pub previous_hash: String,
    pub poh_record_hash: String,
    pub hash: String,
}

impl Block {
    /// Create a new block.
    pub fn new(
        height: u64,
        slot: u64,
        epoch: u64,
        timestamp_ms: u64,
        transactions: Vec<Transaction>,
        validator: &str,
        previous_hash: &str,
        poh_record_hash: &str,
    ) -> Self {
        let mut block: Block = Self {
            height,
            slot,
            epoch,
            timestamp_ms,
            transactions,
            validator: validator.to_string(),
            previous_hash: previous_hash.to_string(),
            poh_record_hash: poh_record_hash.to_string(),
            hash: String::new(),
        };
        // Calculate the block hash.
        block.hash = block.calculate_hash();
        return block;
    }

    /// Calculate the hash of the block.
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        // Hash the block header.
        hasher.update(format!(
            "{}{}{}{}{}{}",
            self.height,
            self.slot,
            self.epoch,
            self.timestamp_ms,
            self.validator,
            self.previous_hash,
        ));
        // Hash the PoH record hash
        hasher.update(&self.poh_record_hash);
        // Hash the transactions
        for tx in &self.transactions {
            hasher.update(&tx.id);
            hasher.update(&tx.signature);
        }
        return format!("{:x}", hasher.finalize());
    }

    /// Verify the integrity of the block.
    pub fn verify(&self) -> bool {
        // Check if the hash is valid.
        let calculated_hash: String = self.calculate_hash();
        if calculated_hash != self.hash {
            return false;
        }
        // Verify all transactions in the block.
        for tx in &self.transactions {
            if !tx.verify() {
                return false;
            }
        }
        return true;
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        return write!(
            f,
            "Block #{} (Slot {}, Epoch {}): {} transactions, Validator: {}, Hash: {}",
            self.height,
            self.slot,
            self.epoch,
            self.transactions.len(),
            self.validator,
            self.hash
        );
    }
}
