use super::core::PoHRecord;

use super::constants::{SLOTS_PER_EPOCH, TICKS_PER_SLOT};

use sha2::{Digest, Sha256};

/// Verifies the integrity of the PoH chain by checking two properties:
///
/// 1. The hash chain is valid: each record's hash is the SHA-256 of the previous record's hash
///    plus the optional event data.
/// 2. The sequence numbers are valid: each record's tick index is one greater than the previous
///    record's tick index, and the slot index and epoch are correctly calculated.
///
/// # Parameters
/// - `records`: The slice of `PoHRecord`s to be verified.
///
/// # Returns
/// `true` if the chain is valid, `false` otherwise.
pub fn verify_chain(records: &[PoHRecord]) -> bool {
    if records.is_empty() {
        return false;
    }

    for window in records.windows(2) {
        let prev: &PoHRecord = &window[0];
        let curr: &PoHRecord = &window[1];

        // Verify hash chain.
        let mut hasher = Sha256::new();
        hasher.update(&prev.hash);

        if let Some(ref evt) = curr.event {
            hasher.update(Sha256::digest(evt));
        }

        let mut expected_hash: [u8; 32] = [0u8; 32];

        expected_hash.copy_from_slice(&hasher.finalize());

        if expected_hash != curr.hash {
            return false;
        }

        // Verify sequence numbers.
        if curr.tick_index != prev.tick_index + 1
            || curr.slot_index != curr.tick_index / TICKS_PER_SLOT
            || curr.epoch != curr.tick_index / (TICKS_PER_SLOT * SLOTS_PER_EPOCH)
        {
            return false;
        }
    }
    return true;
}

#[cfg(test)]
use super::constants::TICK_DURATION_US;
/// Verifies that the timestamps in the given records are monotonically increasing and within
/// a reasonable tolerance of the expected values.
///
/// The tolerance is set to half of the tick duration (0.5 ms) to account for minor
/// variations in the timing of the PoH generation.
///
/// # Parameters
/// - `records`: A slice of `PoHRecord`s to verify.
///
/// # Returns
/// `true` if the timestamps are valid, `false` otherwise.
#[cfg(test)]
pub fn verify_timestamps(records: &[PoHRecord]) -> bool {
    if records.is_empty() {
        return true;
    }

    let first_timestamp: u64 = records[0].timestamp_ms;

    for (i, record) in records.iter().enumerate() {
        let expected_timestamp: u64 = first_timestamp + (i as u64 * TICK_DURATION_US / 1000);
        let allowed_drift: u64 = TICK_DURATION_US / 2000; // ~0.5 ms tolerance.

        if record.timestamp_ms < expected_timestamp.saturating_sub(allowed_drift)
            || record.timestamp_ms > expected_timestamp + allowed_drift
        {
            return false;
        }
    }
    return true;
}
