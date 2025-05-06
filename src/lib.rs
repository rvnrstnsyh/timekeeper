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

// 0: SHA256 (default) 1: BLAKE3
pub static mut DEFAULT_HASH: u8 = 0;
// Number of seconds per day.
pub const DEFAULT_SECONDS_PER_DAY: u64 = 24 * 60 * 60;
// Number of ticks per second.
pub const DEFAULT_TICKS_PER_SECOND: u64 = 160;
// Number of ticks per day.
pub const DEFAULT_TICKS_PER_DAY: u64 = DEFAULT_TICKS_PER_SECOND * DEFAULT_SECONDS_PER_DAY;
// Tick duration in milliseconds (1000 / 160 = 6.25ms approximated as 6ms base).
pub const DEFAULT_MS_PER_TICK: u64 = 1_000 / DEFAULT_TICKS_PER_SECOND;
// Additional timing tolerance per tick in microseconds (0.25ms).
pub const DEFAULT_US_TOLERANCE_PER_TICK: u64 = 250;
// Final tick duration in microseconds: ~6ms + 0.25ms = 6250μs.
pub const DEFAULT_US_PER_TICK: u64 = (DEFAULT_MS_PER_TICK * 1000) + DEFAULT_US_TOLERANCE_PER_TICK;
// Number of ticks per slot (64 ticks = 1 slot).
pub const DEFAULT_TICKS_PER_SLOT: u64 = 64;
// GCP n1-standard hardware and also a xeon e5-2520 v4 are about this rate of hashes/s.
pub const DEFAULT_HASHES_PER_SECOND: u64 = 2_000_000;
// Number of hashes per tick.
pub const DEFAULT_HASHES_PER_TICK: u64 = DEFAULT_HASHES_PER_SECOND / DEFAULT_TICKS_PER_SECOND;
// Expected duration of a slot in seconds.
pub const DEFAULT_S_PER_SLOT: f64 = DEFAULT_TICKS_PER_SLOT as f64 / DEFAULT_TICKS_PER_SECOND as f64;
// Expected duration of a slot (400 milliseconds).
pub const DEFAULT_MS_PER_SLOT: u64 = 1_000 * DEFAULT_TICKS_PER_SLOT / DEFAULT_TICKS_PER_SECOND;
// Number of slots per epoch (432000 slots = 1 epoch).
pub const DEFAULT_SLOTS_PER_EPOCH: u64 = 2 * DEFAULT_TICKS_PER_DAY / DEFAULT_TICKS_PER_SLOT;
// 1 Dev Epoch = 400 ms * 8192 ~= 55 minutes.
pub const DEFAULT_DEV_SLOTS_PER_EPOCH: u64 = 8_192;
// leader schedule is governed by this.
pub const DEFAULT_NUM_CONSECUTIVE_LEADER_SLOTS: u64 = 4;
// Channel capacity for the PoH thread.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1_000;
// Batch size for sending PoH records.
pub const DEFAULT_BATCH_SIZE: usize = 64;
// Performance optimization constants.
pub const DEFAULT_SPINLOCK_THRESHOLD_US: u64 = 250; // Use spinlock for precise timing under threshold

// Blockchain.
pub const DEFAULT_GENESIS_STAKE: u64 = 1000;
pub const DEFAULT_MAX_TRANSACTIONS_PER_BLOCK: usize = 300;
pub const DEFAULT_MIN_TRANSACTIONS_PER_BLOCK: usize = 1;
pub const DEFAULT_REWARD_PER_BLOCK: u64 = 5;
