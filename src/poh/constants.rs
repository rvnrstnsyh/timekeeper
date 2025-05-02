// Constants used by the Proof of History (PoH) and its generator.
pub const TICKS_PER_SLOT: u64 = 64;
pub const SLOTS_PER_EPOCH: u64 = 432_000;
pub const TICK_DURATION_US: u64 = 6_250; // 6.25 ms = 6250 µs.
pub const BATCH_SIZE: usize = 64; // Batching for sending display data.
