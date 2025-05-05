pub mod blockchain {
    pub mod block;
    pub mod processor;
    pub mod state;
    pub mod transaction;
    pub mod validator;
}
pub mod helpers {
    pub mod args;
    pub mod io;
    pub mod serialization;
}
pub mod poh {
    pub mod core;
    pub mod hash;
    pub mod thread;
    pub mod verifier;
}

// Proof of History (PoH) timing constants:
// 1. 12500 hashes = 1 tick
// 2. 1 tick = 6.25ms
// 3. 64 ticks = 1 slot
// 4. 432000 slots = 1 epoch
// 5. 1 epoch = 2 days (48 hours)

// Tick duration in microseconds (6.25ms = 6250μs)
pub const TICK_DURATION_US: u64 = 6_250;
// Number of ticks per slot (64 ticks = 1 slot)
pub const TICKS_PER_SLOT: u64 = 64;
// Slot duration in milliseconds (400ms = 0.4s)
pub const SLOT_DURATION_MS: u64 = 400;
// Number of slots per epoch (432000 slots = 1 epoch)
pub const SLOTS_PER_EPOCH: u64 = 432_000;
// Number of hashes per tick
pub const HASHES_PER_TICK: u64 = 12_500;
// Channel capacity for the PoH thread
pub const CHANNEL_CAPACITY: usize = 1000;
// Batch size for sending PoH records
pub const BATCH_SIZE: usize = 64;
// Performance optimization constants
pub const SPINLOCK_THRESHOLD_US: u64 = 250; // Use spinlock for precise timing under threshold

// Blockchain.
pub const GENESIS_STAKE: u64 = 1000;
pub const MIN_TRANSACTIONS_PER_BLOCK: usize = 1;
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 300;
pub const REWARD_PER_BLOCK: u64 = 5;
