use std::fmt::{Display, Formatter, Result};
use std::time::Instant;

use crate::helpers::serialization;
use crate::poh::hash;
use crate::poh::verifier;
use crate::{DEFAULT_HASHES_PER_TICK, DEFAULT_SLOTS_PER_EPOCH, DEFAULT_TICKS_PER_SLOT};

use hex::encode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoHRecord {
    pub tick_index: u64,
    pub slot_index: u64,
    pub epoch_index: u64,
    #[serde(with = "serialization")]
    pub hash: [u8; 32],
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<u8>>,
}

impl Display for PoHRecord {
    /// Formats the `PoHRecord` in a human-readable way.
    ///
    /// Example output:
    /// "Epoch 0, Slot 0, Tick 0, Timestamp 0ms, Hash 0x000000000000000..."
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let _event_desc: String = match &self.event {
            Some(data) => format!("Event: {} bytes", data.len()),
            None => "No Event".to_string(),
        };
        write!(
            f,
            "Epoch {}, Slot {}, Tick {}, Timestamp {}ms, Hash 0x{}...",
            self.epoch_index,
            self.slot_index,
            self.tick_index,
            self.timestamp_ms,
            &encode(self.hash)[..17]
        )
    }
}

pub struct PoH {
    current_hash: [u8; 32],
    tick_count: u64,
    slot_count: u64,
    epoch_count: u64,
    start_time: Instant, // Using Instant for more accurate time measurement.
}

impl PoH {
    /// Create a new Proof of History (PoH) records with the given seed data.
    ///
    /// The seed data is used to generate the first hash in the records.
    /// The PoH records starts with tick index 0, slot index 0, and epoch 0.
    /// The timestamp is set to the current system time.
    ///
    /// # Parameters
    /// - `seed`: The data used to generate the first hash in the records.
    ///
    /// # Returns
    /// A `PoH` object initialized with the given seed data.
    pub fn new(seed: &[u8]) -> Self {
        let current_hash: [u8; 32] = hash::hash(seed);
        Self {
            current_hash,
            tick_count: 0,
            slot_count: 0,
            epoch_count: 0,
            start_time: Instant::now(),
        }
    }

    /// PoH (Proof of History) by one tick.
    ///
    /// This function does not insert any event data and simply increments the tick count.
    /// It updates the current hash with the previous hash, increments the tick count, and
    /// returns a new PoHRecord. The function also updates slot and epoch counts based on the
    /// number of ticks.
    ///
    /// # Returns
    /// A new `PoHRecord` with the updated hash, tick index, slot index, epoch, and timestamp.
    pub fn next_tick(&mut self) -> PoHRecord {
        self.core(None)
    }

    /// Insert an event into the PoH (Proof of History) records.
    ///
    /// This function adds the provided event data to the PoH records by advancing the records
    /// with the event data included in the hash calculation. It increments the tick count
    /// and returns a new PoHRecord containing the updated state.
    ///
    /// # Parameters
    /// - `event_data`: A byte slice representing the event data to be inserted into the records.
    ///
    /// # Returns
    /// A `PoHRecord` with the updated tick index, slot index, epoch, hash, timestamp, and
    /// the inserted event data.
    pub fn insert_event(&mut self, event_data: &[u8]) -> PoHRecord {
        self.core(Some(event_data))
    }

    /// PoH (Proof of History) by either generating a new tick or inserting an event.
    ///
    /// This function updates the current hash with the previous hash and optional event data,
    /// increments the tick count, and returns a new PoHRecord. The function also updates slot and
    /// epoch counts based on the number of ticks.
    ///
    /// # Parameters
    ///  `event_data`: Optionally, a byte slice representing event data to be included in the
    ///   hash. If `Some`, the event data is hashed along with the current hash,
    ///   otherwise only the current hash is used.
    ///
    /// # Returns
    /// A `PoHRecord` containing the updated tick index, slot index, epoch, hash, timestamp, and
    /// optional event data.
    fn core(&mut self, event_data: Option<&[u8]>) -> PoHRecord {
        if let Some(event) = event_data {
            self.current_hash = hash::hash_with_data(&self.current_hash, event);
        }

        // Now extend the hash chain by DEFAULT_HASHES_PER_TICK hashes using centralized function.
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

        self.tick_count += 1;

        if self.tick_count % DEFAULT_TICKS_PER_SLOT == 0 {
            self.slot_count += 1;
        }
        if slot_index % DEFAULT_SLOTS_PER_EPOCH == 0 && self.tick_count % DEFAULT_TICKS_PER_SLOT == 0 {
            self.epoch_count = epoch_index;
            self.slot_count = 0;
        }

        return record;
    }

    pub fn verify_records(records: &[PoHRecord]) -> bool {
        verifier::verify_records(records)
    }

    pub fn verify_timestamps(records: &[PoHRecord]) -> bool {
        verifier::verify_timestamps(records, true)
    }
}
