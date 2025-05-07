use std::fmt::{Display, Formatter, Result};
use std::time::Instant;

use crate::types::{PoH, PoHRecord};

use lib::utils::hash;
use lib::{DEFAULT_HASHES_PER_TICK, DEFAULT_SLOTS_PER_EPOCH, DEFAULT_TICKS_PER_SLOT, DEFAULT_US_PER_TICK};

use hex::encode;

impl Display for PoHRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let _event_desc: String = match &self.event {
            Some(data) => format!("Event: {} bytes", data.len()),
            None => "No Event".to_string(),
        };
        return write!(
            f,
            "Epoch {}, Slot {}, Tick {}, Timestamp {}ms, Hash 0x{}...",
            self.epoch_index,
            self.slot_index,
            self.tick_index,
            self.timestamp_ms,
            &encode(self.hash)[..17]
        );
    }
}

impl PoH {
    pub fn new(seed: &[u8]) -> Self {
        let current_hash: [u8; 32] = hash::hash(seed);
        return Self {
            current_hash,
            tick_count: 0,
            slot_count: 0,
            epoch_count: 0,
            start_time: Instant::now(),
        };
    }

    pub fn next_tick(&mut self) -> PoHRecord {
        return self.core(None);
    }

    pub fn insert_event(&mut self, event_data: &[u8]) -> PoHRecord {
        return self.core(Some(event_data));
    }

    fn core(&mut self, event_data: Option<&[u8]>) -> PoHRecord {
        if let Some(event) = event_data {
            self.current_hash = hash::hash_with_data(&self.current_hash, event);
        }

        self.current_hash = hash::extend_hash_chain(&self.current_hash, DEFAULT_HASHES_PER_TICK);

        let tick_index: u64 = self.tick_count;
        let slot_index: u64 = tick_index / DEFAULT_TICKS_PER_SLOT;
        let epoch_index: u64 = slot_index / DEFAULT_SLOTS_PER_EPOCH;
        let record: PoHRecord = PoHRecord {
            tick_index,
            slot_index,
            epoch_index,
            hash: self.current_hash,
            timestamp_ms: self.start_time.elapsed().as_millis() as u64,
            event: event_data.map(|d| d.to_vec()),
        };

        self.tick_count = self.tick_count.checked_add(1).expect("tick_count overflow");

        if self.tick_count % DEFAULT_TICKS_PER_SLOT == 0 {
            self.slot_count = self.slot_count.checked_add(1).expect("slot_count overflow");
        }
        if slot_index % DEFAULT_SLOTS_PER_EPOCH == 0 && self.tick_count % DEFAULT_TICKS_PER_SLOT == 0 {
            self.epoch_count = epoch_index;
            self.slot_count = 0;
        }
        return record;
    }

    pub fn verify_records(records: &[PoHRecord]) -> bool {
        if records.is_empty() {
            return false;
        }

        for window in records.windows(2) {
            let prev: &PoHRecord = &window[0];
            let curr: &PoHRecord = &window[1];
            let event_data: Option<&[u8]> = curr.event.as_deref();

            if !hash::verify_hash_chain(&prev.hash, &curr.hash, DEFAULT_HASHES_PER_TICK, event_data) {
                return false;
            }

            // Verify sequence numbers.
            let tick_index_valid: bool = curr.tick_index == prev.tick_index.saturating_add(1);
            let slot_index_valid: bool = curr.slot_index == curr.tick_index / DEFAULT_TICKS_PER_SLOT;
            let epoch_valid: bool = curr.epoch_index == curr.tick_index / (DEFAULT_TICKS_PER_SLOT * DEFAULT_SLOTS_PER_EPOCH);

            if !(tick_index_valid && slot_index_valid && epoch_valid) {
                return false;
            }
        }
        return true;
    }

    pub fn verify_timestamps(records: &[PoHRecord], log_failures: bool) -> bool {
        if records.is_empty() {
            return false;
        }

        let first_timestamp: u64 = records[0].timestamp_ms;
        // let mut all_valid: bool = true;

        for (i, record) in records.iter().enumerate() {
            let timestamp: u64 = record.timestamp_ms;
            let expected_timestamp: u64 = first_timestamp.saturating_add((i as u64).checked_mul(DEFAULT_US_PER_TICK).unwrap_or(0) / 1000);
            // Adjust tolerance based on whether this is an event tick.
            let allowed_drift: u64 = 8; // ~8ms tolerance, relaxed.
            // Ensure we don't underflow.
            let lower_bound: u64 = expected_timestamp.saturating_sub(allowed_drift);
            let upper_bound: u64 = expected_timestamp.saturating_add(allowed_drift);

            let too_early: bool = timestamp < lower_bound;
            let too_late: bool = timestamp > upper_bound;

            if too_early || too_late {
                if log_failures {
                    println!(
                        "Timestamp mismatch at record {}: actual={}, expected={}, drift={}, allowed=~{}",
                        i,
                        timestamp,
                        expected_timestamp,
                        if too_early {
                            lower_bound.saturating_sub(timestamp)
                        } else {
                            timestamp.saturating_sub(upper_bound)
                        },
                        allowed_drift
                    );
                }
                // all_valid = false;
                // Optional: return immediately on first failure
                return false;
            }
        }
        return true;
    }
}
