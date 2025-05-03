// Proof of History (PoH).
pub const TICKS_PER_SLOT: u64 = 64;
pub const TICK_DURATION_US: u64 = 6_250; // 6.25 ms = 6250 µs.
pub const SLOT_DURATION_MS: u64 = 400; // 400ms per slot (64 ticks at 6.25ms)
pub const SLOTS_PER_EPOCH: u64 = 432_000;
pub const BATCH_SIZE: usize = 64; // Batching for sending display data.

// Blockchain.
pub const GENESIS_STAKE: u64 = 1000;
pub const MIN_TRANSACTIONS_PER_BLOCK: usize = 1;
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 300;
pub const REWARD_PER_BLOCK: u64 = 5;
