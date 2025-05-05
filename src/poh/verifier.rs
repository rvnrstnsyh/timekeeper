use crate::poh::core::PoHRecord;
use crate::poh::hash;
use crate::{HASHES_PER_TICK, SLOTS_PER_EPOCH, TICK_DURATION_US, TICKS_PER_SLOT};

/// Verifies the integrity of the PoH records by checking two properties:
///
/// 1. The hash records is valid: each record's hash is the result of applying
///    HASHES_PER_TICK hashes to the previous record's hash plus optional event data.
/// 2. The sequence numbers are valid: each record's tick index is one greater than
///    the previous record's tick index, and the slot index and epoch are correctly calculated.
///
/// # Parameters
/// - `records`: The slice of `PoHRecord`s to be verified.
///
/// # Returns
/// `true` if the records is valid, `false` otherwise.
pub fn verify_records(records: &[PoHRecord]) -> bool {
    if records.is_empty() {
        return false;
    }

    for window in records.windows(2) {
        let prev: &PoHRecord = &window[0];
        let curr: &PoHRecord = &window[1];

        // Verify the hash chain using centralized verification function.
        let event_data: Option<&[u8]> = curr.event.as_deref();
        if !hash::verify_hash_chain(&prev.hash, &curr.hash, HASHES_PER_TICK, event_data) {
            return false;
        }

        // Verify sequence numbers.
        let tick_index_valid: bool = curr.tick_index == prev.tick_index + 1;
        let slot_index_valid: bool = curr.slot_index == curr.tick_index / TICKS_PER_SLOT;
        let epoch_valid: bool = curr.epoch == curr.tick_index / (TICKS_PER_SLOT * SLOTS_PER_EPOCH);

        if !(tick_index_valid && slot_index_valid && epoch_valid) {
            return false;
        }
    }

    true
}

/// Verifies that the timestamps in the given PoH records are valid by comparing
/// each timestamp to the expected timestamp based on the tick duration and the
/// first timestamp in the records. For event ticks, a larger tolerance is allowed
/// to account for the extra time taken to process events.
///
/// # Parameters
/// - `records`: The slice of `PoHRecord`s to be verified.
/// - `log_failures`: If `true`, logs a message for each failure.
///
/// # Returns
/// `true` if all timestamps are valid, `false` otherwise.
pub fn verify_timestamps(records: &[PoHRecord], log_failures: bool) -> bool {
    if records.is_empty() {
        return false;
    }

    let first_timestamp: u64 = records[0].timestamp_ms;
    // let mut all_valid: bool = true;

    for (i, record) in records.iter().enumerate() {
        let timestamp: u64 = record.timestamp_ms;
        let expected_timestamp: u64 = first_timestamp + (i as u64 * TICK_DURATION_US / 1000);
        // Adjust tolerance based on whether this is an event tick.
        let allowed_drift: u64 = 8; // ~8ms tolerance, relaxed.
        // Ensure we don't underflow.
        let lower_bound: u64 = expected_timestamp.saturating_sub(allowed_drift);
        let upper_bound: u64 = expected_timestamp + allowed_drift;

        let too_early: bool = timestamp < lower_bound;
        let too_late: bool = timestamp > upper_bound;

        if too_early || too_late {
            if log_failures {
                println!(
                    "Timestamp mismatch at record {}: actual={}, expected={}, drift={}, allowed=~{}",
                    i,
                    timestamp,
                    expected_timestamp,
                    if too_early { lower_bound - timestamp } else { timestamp - upper_bound },
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
