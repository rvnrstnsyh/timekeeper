use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::blockchain::block::Block;
use crate::blockchain::state::BlockchainState;
use crate::blockchain::transaction::Transaction;
use crate::blockchain::validator::Validator;
use crate::poh::constants::SLOT_DURATION_MS;
use crate::poh::core::{PoH, PoHRecord};

/// Blockchain processor that connects to the PoH generator.
#[derive(Debug, Clone)]
pub struct BlockchainProcessor {
    pub state: Arc<Mutex<BlockchainState>>,
    pub transaction_queue: Arc<Mutex<Vec<Transaction>>>,
}

impl BlockchainProcessor {
    /// Create a new blockchain processor.
    pub fn new() -> Self {
        return Self {
            state: Arc::new(Mutex::new(BlockchainState::new())),
            transaction_queue: Arc::new(Mutex::new(Vec::new())),
        };
    }

    /// Submit a transaction to the blockchain.
    pub fn submit_transaction(&self, tx: Transaction) -> bool {
        // Add to the transaction queue.
        let mut queue: MutexGuard<'_, Vec<Transaction>> = self.transaction_queue.lock().unwrap();
        queue.push(tx);
        return true;
    }

    /// Create a transaction and submit it to the blockchain.
    pub fn create_transaction(&self, sender: &str, recipient: &str, amount: u64) -> bool {
        let tx: Transaction = Transaction::new(sender, recipient, amount);
        return self.submit_transaction(tx);
    }

    /// Process pending transactions and integrate with PoH.
    pub fn process_transactions(&self, poh: &mut PoH) -> Vec<PoHRecord> {
        let mut poh_records: Vec<PoHRecord> = Vec::new();
        // Process pending transactions.
        let mut queue: MutexGuard<'_, Vec<Transaction>> = self.transaction_queue.lock().unwrap();
        let transactions: Vec<Transaction> = queue.drain(..).collect();
        // Add transactions to blockchain state.
        let mut state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();

        for tx in transactions {
            if state.add_transaction(tx.clone()) {
                // Insert transaction as an event in PoH.
                let record: PoHRecord = poh.insert_event(&tx.to_bytes());
                poh_records.push(record);
            }
        }
        return poh_records;
    }

    /// Process a new slot and generate a block if needed.
    pub fn process_slot(&self, poh_record: &PoHRecord) -> Option<Block> {
        let mut state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();
        // Update the current slot and epoch.
        state.current_slot = poh_record.slot_index;
        state.current_epoch = poh_record.epoch;
        // Generate a new block for this slot.
        return state.generate_block(poh_record);
    }

    /// Start the blockchain processor in a separate thread.
    pub fn start(&self, poh: Arc<Mutex<PoH>>, running: Arc<AtomicBool>) -> () {
        let state: Arc<Mutex<BlockchainState>> = self.state.clone();
        let tx_queue: Arc<Mutex<Vec<Transaction>>> = self.transaction_queue.clone();

        thread::spawn(move || {
            let mut last_slot: u64 = 0;
            let start_time: Instant = Instant::now();

            println!("Blockchain thread started and monitoring running flag");

            while running.load(Ordering::Relaxed) {
                // Simulate slot timing.
                let elapsed_ms: u64 = start_time.elapsed().as_millis() as u64;
                let current_slot: u64 = elapsed_ms / SLOT_DURATION_MS;

                if current_slot > last_slot {
                    // Get PoH record for this slot.
                    let poh_record: PoHRecord = {
                        let mut poh_guard: MutexGuard<'_, PoH> = match poh.lock() {
                            Ok(guard) => guard,
                            Err(_) => {
                                println!("PoH mutex was poisoned, exiting blockchain thread");
                                break;
                            }
                        };
                        let record: PoHRecord = poh_guard.next_tick();
                        record
                    }; // PoH lock is released here.

                    // Process queued transactions.
                    {
                        let mut state: MutexGuard<'_, BlockchainState> = state.lock().unwrap();
                        let mut queue: MutexGuard<'_, Vec<Transaction>> = tx_queue.lock().unwrap();
                        // Process transactions from queue to pending_transactions.
                        let transactions: Vec<Transaction> = queue.drain(..).collect();
                        for tx in transactions {
                            state.add_transaction(tx);
                        }
                        // Generate block.
                        if let Some(block) = state.generate_block(&poh_record) {
                            println!("New block generated in slot {}: {}", current_slot, block);
                        } else {
                            println!("No block generated for slot {}", current_slot);
                        }
                    }
                    last_slot = current_slot;
                }

                // Check the running flag more frequently.
                thread::sleep(Duration::from_millis(10));
                // Extra check to ensure we can break out of the loop even between slots.
                if !running.load(Ordering::Relaxed) {
                    println!("Blockchain thread shutting down between slots");
                    break;
                }
            }
            println!("Blockchain thread is shutting down");
        });
    }

    /// Get the current blockchain state.
    pub fn get_state(&self) -> BlockchainState {
        return self.state.lock().unwrap().clone();
    }

    /// Get a block by its height.
    pub fn get_block(&self, height: u64) -> Option<Block> {
        let state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();
        return state.blocks.iter().find(|b| b.height == height).cloned();
    }

    /// Get blocks in a range.
    pub fn get_blocks(&self, start: u64, end: u64) -> Vec<Block> {
        let state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();
        return state
            .blocks
            .iter()
            .filter(|b| b.height >= start && b.height < end)
            .cloned()
            .collect();
    }

    /// Get a validator by address.
    pub fn get_validator(&self, address: &str) -> Option<Validator> {
        let state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();
        return state.validators.get(address).cloned();
    }

    /// Get all validators.
    pub fn get_validators(&self) -> Vec<Validator> {
        let state: MutexGuard<'_, BlockchainState> = self.state.lock().unwrap();
        return state.validators.values().cloned().collect();
    }
}
