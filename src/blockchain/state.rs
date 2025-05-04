use std::collections::HashMap;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::blockchain::validator::Validator;
use crate::constants::{GENESIS_STAKE, MAX_TRANSACTIONS_PER_BLOCK, MIN_TRANSACTIONS_PER_BLOCK, REWARD_PER_BLOCK};
use crate::poh::core::PoHRecord;

use rand::Rng;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};

/// Represents the blockchain state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainState {
    pub blocks: Vec<Block>,
    pub validators: HashMap<String, Validator>,
    pub pending_transactions: Vec<Transaction>,
    pub current_slot: u64,
    pub current_epoch: u64,
    pub last_block_hash: String,
}

impl BlockchainState {
    /// Create a new blockchain state with genesis block.
    pub fn new() -> Self {
        let mut state: BlockchainState = Self {
            blocks: Vec::new(),
            validators: HashMap::new(),
            pending_transactions: Vec::new(),
            current_slot: 0,
            current_epoch: 0,
            last_block_hash: "0".repeat(64).to_string(),
        };
        // Create some initial validators.
        let validator_addresses: Vec<String> = vec![
            "validator-1".to_string(),
            "validator-2".to_string(),
            "validator-3".to_string(),
            "validator-4".to_string(),
        ];
        for address in validator_addresses {
            state.validators.insert(address.clone(), Validator::new(&address, GENESIS_STAKE));
        }
        return state;
    }

    /// Add a new transaction to the pending pool.
    pub fn add_transaction(&mut self, transaction: Transaction) -> bool {
        // Verify the transaction.
        if !transaction.verify() {
            return false;
        }

        // TODO: Check if the sender has enough balance later.

        self.pending_transactions.push(transaction);
        return true;
    }

    /// Select a validator for the current slot using PoS.
    pub fn select_validator(&self) -> Option<String> {
        if self.validators.is_empty() {
            return None;
        }

        // Calculate total stake.
        let total_stake: u64 = self.validators.values().map(|v| v.stake).sum();
        // Get a random number between 0 and total_stake.
        let mut rng: ThreadRng = rand::rng();
        let mut selection_point: u64 = rng.random_range(0..total_stake);

        // Weighted selection based on stake.
        for (address, validator) in &self.validators {
            if selection_point < validator.stake {
                return Some(address.clone());
            }
            selection_point -= validator.stake;
        }
        // Fallback to the first validator if something went wrong.
        return Some(self.validators.keys().next().unwrap().clone());
    }

    /// Generate a new block for the current slot with pending transactions.
    pub fn generate_block(&mut self, poh_record: &PoHRecord) -> Option<Block> {
        // Remove the empty transaction check to allow block creation without transactions
        // if self.pending_transactions.is_empty() {
        //     // No transactions to include in the block
        //     return None;
        // }

        // Select a validator for this slot.
        let validator_address: String = self.select_validator()?;
        // Get transactions for this block, even if empty.
        let transactions: Vec<Transaction> = if !self.pending_transactions.is_empty() {
            let tx_count: usize = std::cmp::min(
                rand::rng().random_range(MIN_TRANSACTIONS_PER_BLOCK..=MAX_TRANSACTIONS_PER_BLOCK),
                self.pending_transactions.len(),
            );
            self.pending_transactions.drain(0..tx_count).collect()
        } else {
            Vec::new() // Empty transactions array.
        };
        // Create the new block.
        let block: Block = Block::new(
            self.blocks.len() as u64,
            self.current_slot,
            self.current_epoch,
            poh_record.timestamp_ms,
            transactions,
            &validator_address,
            &self.last_block_hash,
            &hex::encode(poh_record.hash),
        );
        // Update the validator's stats.
        if let Some(validator) = self.validators.get_mut(&validator_address) {
            validator.blocks_produced += 1;
            validator.last_active_slot = self.current_slot;
            validator.rewards_earned += REWARD_PER_BLOCK;
            validator.stake += REWARD_PER_BLOCK; // Automatically stake the rewards.
        }
        // Update blockchain state.
        self.blocks.push(block.clone());
        self.last_block_hash = block.hash.clone();
        return Some(block);
    }
}

impl Default for BlockchainState {
    fn default() -> Self {
        Self::new()
    }
}
