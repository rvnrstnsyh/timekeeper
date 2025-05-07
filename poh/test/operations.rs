#[cfg(test)]
mod operations {
    use std::sync::mpsc::Receiver;
    use std::time::{Duration, Instant};

    use lib::utils::hash;
    use lib::{DEFAULT_HASHES_PER_TICK, DEFAULT_MS_PER_TICK, DEFAULT_SLOTS_PER_EPOCH, DEFAULT_TICKS_PER_SLOT, DEFAULT_US_PER_TICK};

    use poh::thread;
    use poh::types::{PoH, PoHRecord};

    #[test]
    fn test_poh_record_construction() {
        let seed: [u8; 64] = [0u8; 64];
        let mut poh: PoH = PoH::new(&seed);

        let record1: PoHRecord = poh.next_tick();
        let record2: PoHRecord = poh.next_tick();

        // Ensure records have consecutive tick indices.
        assert_eq!(record1.tick_index + 1, record2.tick_index);
        // Ensure slot calculation is correct.
        assert_eq!(record1.slot_index, record1.tick_index / DEFAULT_TICKS_PER_SLOT);
        assert_eq!(record2.slot_index, record2.tick_index / DEFAULT_TICKS_PER_SLOT);
        // Ensure epoch calculation is correct.
        assert_eq!(record1.epoch_index, record1.slot_index / DEFAULT_SLOTS_PER_EPOCH);
        assert_eq!(record2.epoch_index, record2.slot_index / DEFAULT_SLOTS_PER_EPOCH);
    }

    #[test]
    fn test_hash_chain_extension() {
        let seed: [u8; 32] = [1u8; 32]; // Some seed data.
        let iterations: u64 = 10;
        // Test hash chain extension.
        let result: [u8; 32] = hash::extend_hash_chain(&seed, iterations);
        // Verify by manually applying hash iterations.
        let mut expected: [u8; 32] = seed;

        for _ in 0..iterations {
            expected = hash::hash(&expected);
        }

        assert_eq!(result, expected, "Hash chain extension produced incorrect result.");
    }

    #[test]
    fn test_hash_chain_verification() {
        let seed: [u8; 32] = [1u8; 32]; // Initial hash.
        let iterations: u64 = DEFAULT_HASHES_PER_TICK;
        let event_data: &'static [u8; 10] = b"Test event";
        // Create a valid hash chain with event.
        let mut current_hash: [u8; 32] = hash::hash_with_data(&seed, event_data);

        current_hash = hash::extend_hash_chain(&current_hash, iterations);
        // Verify the valid hash chain.
        assert!(
            hash::verify_hash_chain(&seed, &current_hash, iterations, Some(event_data)),
            "Valid hash chain verification failed."
        );

        // Modify hash and ensure verification fails.
        let mut bad_hash: [u8; 32] = current_hash;
        bad_hash[0] ^= 0xFF; // Corrupt the hash.

        assert!(
            !hash::verify_hash_chain(&seed, &bad_hash, iterations, Some(event_data)),
            "Corrupted hash chain verification didn't fail."
        );
    }

    #[test]
    fn test_event_insertion() {
        let seed: [u8; 64] = [0u8; 64];
        let mut poh: PoH = PoH::new(&seed);

        let tick1: PoHRecord = poh.next_tick(); // Normal tick.
        let event_data: &'static str = "Test event data";
        let tick2: PoHRecord = poh.insert_event(event_data.as_bytes()); // Tick with event.
        let tick3: PoHRecord = poh.next_tick(); // Normal tick.

        // Check that event was stored.
        assert!(tick2.event.is_some());
        assert_eq!(tick2.event.clone().unwrap(), event_data.as_bytes());
        // Check that non-event ticks don't have events.
        assert!(tick1.event.is_none());
        assert!(tick3.event.is_none());
        // Verify hash chain integrity across all ticks.
        let records: Vec<PoHRecord> = vec![tick1, tick2, tick3];
        assert!(PoH::verify_records(&records), "Records with event failed verification.");
    }

    #[test]
    fn test_slot_transition() {
        let seed: [u8; 64] = [0u8; 64];
        let mut poh: PoH = PoH::new(&seed);
        let mut records: Vec<PoHRecord> = Vec::with_capacity((DEFAULT_TICKS_PER_SLOT + 5) as usize);

        // Generate ticks across a slot boundary.
        for _ in 0..DEFAULT_TICKS_PER_SLOT + 5 {
            records.push(poh.next_tick());
        }

        let last_tick: &PoHRecord = &records[DEFAULT_TICKS_PER_SLOT as usize - 1];
        let first_tick: &PoHRecord = &records[DEFAULT_TICKS_PER_SLOT as usize];

        // Verify slot transition.
        // Slot indexing starts at 0, so the last tick of slot 0 should be at index DEFAULT_TICKS_PER_SLOT-1.
        assert_eq!(last_tick.slot_index, 0, "Last tick of slot 0 has incorrect slot_index.");
        // The first tick of slot 1 should be at index DEFAULT_TICKS_PER_SLOT.
        assert_eq!(first_tick.slot_index, 1, "First tick of slot 1 has incorrect slot_index.");
        // Verify hash chain integrity across slot boundary.
        assert!(PoH::verify_records(&records), "Records across slot boundary failed verification.");
    }

    #[test]
    fn test_poh_thread() {
        let seed: [u8; 64] = [0u8; 64];
        let test_ticks: u64 = 32; // Use a smaller number for reliable testing.

        let start: Instant = Instant::now();
        let rx: Receiver<PoHRecord> = thread::thread(&seed, test_ticks);
        let mut records: Vec<PoHRecord> = Vec::with_capacity(test_ticks as usize);

        while let Ok(record) = rx.recv() {
            records.push(record);
            if records.len() == test_ticks as usize {
                break; // Ensure we exit once we have enough records.
            }
        }

        let elapsed: Duration = start.elapsed();
        // Verify we got the expected number of records.
        assert_eq!(records.len(), test_ticks as usize, "Received incorrect number of records.");
        // Verify hash chain integrity
        assert!(PoH::verify_records(&records), "Thread-generated records failed verification.");
        // Check timing with a very generous tolerance for test environments.
        // Each tick should be ~6.25ms, but we allow more variance due to test environment constraints.
        let expected_duration: Duration = Duration::from_micros(DEFAULT_US_PER_TICK * test_ticks);
        let lower_bound: Duration = expected_duration.mul_f64(0.75); // 75% lower tolerance.
        let upper_bound: Duration = expected_duration.mul_f64(3.0); // 300% upper tolerance.

        if !cfg!(debug_assertions) {
            assert!(
                elapsed >= lower_bound && elapsed <= upper_bound,
                "Thread timing outside acceptable range. Expected ~{:?}, got {:?}.",
                expected_duration,
                elapsed
            );
        }
    }

    #[test]
    fn test_timestamp_consistency() {
        let seed: [u8; 64] = [0u8; 64];
        let mut poh: PoH = PoH::new(&seed);
        let count: usize = 100;
        let mut records: Vec<PoHRecord> = Vec::with_capacity(count);

        for _ in 0..count {
            records.push(poh.next_tick());
        }
        // Verify timestamps are monotonically increasing.
        for i in 1..records.len() {
            assert!(
                records[i].timestamp_ms >= records[i - 1].timestamp_ms,
                "Timestamps not monotonically increasing at index {}.",
                i
            );
        }

        // Check average tick duration is reasonable (without strict assertions).
        let mut total_diff: u64 = 0u64;
        let mut count_diffs: i32 = 0;

        for i in 1..records.len() {
            total_diff += records[i].timestamp_ms - records[i - 1].timestamp_ms;
            count_diffs += 1;
        }
        if count_diffs > 0 {
            let avg_tick_ms: f64 = total_diff as f64 / count_diffs as f64;
            // Allow wide tolerance since test environment timing can vary.
            assert!(avg_tick_ms > 0.0, "Average tick duration should be positive.");
            println!("Average tick duration: {:.3} ms.", avg_tick_ms);
        }
    }

    #[test]
    fn test_corruption_detection() {
        let seed: [u8; 64] = [0u8; 64];
        let mut poh: PoH = PoH::new(&seed);
        let count: usize = 10;
        let mut records: Vec<PoHRecord> = Vec::with_capacity(count);

        for _ in 0..count {
            records.push(poh.next_tick());
        }

        // Verify original records are valid.
        assert!(PoH::verify_records(&records), "Valid records failed verification.");
        // Test various corruption scenarios
        let mut corrupted: Vec<PoHRecord> = records.clone();
        // 1. Corrupt a hash.
        corrupted[5].hash[0] ^= 0xFF;
        assert!(!PoH::verify_records(&corrupted), "Failed to detect hash corruption.");
        // 2. Corrupt tick index.
        corrupted = records.clone();
        corrupted[3].tick_index += 2; // Skip a tick index.
        assert!(!PoH::verify_records(&corrupted), "Failed to detect tick index corruption.");
        // 3. Corrupt slot index.
        corrupted = records.clone();
        corrupted[4].slot_index += 1; // Incorrect slot index.
        assert!(!PoH::verify_records(&corrupted), "Failed to detect slot index corruption.");
        // 4. Corrupt epoch.
        corrupted = records.clone();
        corrupted[5].epoch_index += 1; // Incorrect epoch.
        assert!(!PoH::verify_records(&corrupted), "Failed to detect epoch corruption.");
    }

    #[test]
    fn test_constant_time_eq() {
        // We can't test the actual constant-time property, but we can test correctness.
        let hash1: [u8; 32] = [0u8; 32];
        let hash2: [u8; 32] = [0u8; 32];
        let hash3: [u8; 32] = {
            let mut h: [u8; 32] = [0u8; 32];
            h[31] = 1; // Differs at the last byte.
            h
        };

        // Test the function through verify_hash_chain which uses constant_time_eq.
        assert!(hash::verify_hash_chain(&hash1, &hash2, 0, None), "Equal hashes not recognized as equal.");
        assert!(!hash::verify_hash_chain(&hash1, &hash3, 0, None), "Different hashes not recognized as different.");
    }

    #[test]
    fn test_realistic_poh_operation() {
        let seed: [u8; 64] = [0u8; 64];
        // Use a smaller number of ticks to make the test more reliable.
        let test_ticks: u64 = 128; // 2 slots worth of ticks.

        let start: Instant = Instant::now();
        let rx: Receiver<PoHRecord> = thread::thread(&seed, test_ticks);

        let mut records: Vec<PoHRecord> = Vec::with_capacity(test_ticks as usize);
        let mut last_slot: u64 = 0;
        let mut slot_transitions: i32 = 0;
        let mut counter: u64 = 0;

        while let Ok(record) = rx.recv() {
            if record.slot_index != last_slot {
                slot_transitions += 1;
                last_slot = record.slot_index;
            }
            records.push(record);
            counter += 1;
            if counter >= test_ticks {
                break; // Ensure we exit once we have enough records.
            }
        }

        let elapsed: Duration = start.elapsed();
        // Verify tick count.
        assert_eq!(records.len(), test_ticks as usize, "Incorrect number of ticks generated.");
        // Verify slot transitions (should be 1 for 128 ticks with 64 ticks per slot).
        assert_eq!(slot_transitions, 1, "Incorrect number of slot transitions.");

        // Verify timing with a very generous tolerance for test environments.
        let expected_ms: u64 = (test_ticks as f64 * DEFAULT_MS_PER_TICK as f64) as u64;
        let actual_ms: u64 = elapsed.as_millis() as u64;
        // More lenient tolerance: 75% to 300% of the expected value.
        let lower_bound: u64 = (expected_ms as f64 * 0.75) as u64;
        let upper_bound: u64 = (expected_ms as f64 * 3.0) as u64;

        // Print timing info for debugging
        println!(
            "Expected: ~{} ms, Got: {} ms, Acceptable range: {} ms - {} ms.",
            expected_ms, actual_ms, lower_bound, upper_bound
        );
        // Ensure actual time falls within the tolerance range.
        if !cfg!(debug_assertions) {
            assert!(
                actual_ms >= lower_bound && actual_ms <= upper_bound,
                "Timing outside acceptable range: expected ~{} ms, got {} ms.",
                expected_ms,
                actual_ms
            );
        }
        // Verify integrity.
        assert!(PoH::verify_records(&records), "PoH records failed verification.");
    }

    #[test]
    fn test_hash_rate_constant() {
        // Verify that DEFAULT_HASHES_PER_TICK = 12500 as specified in requirements.
        assert_eq!(DEFAULT_HASHES_PER_TICK, 12500, "DEFAULT_HASHES_PER_TICK should be 12500.");
    }

    #[test]
    fn test_tick_duration_constant() {
        // Verify that DEFAULT_US_PER_TICK is approximately 6250 microseconds (6.25ms).
        assert_eq!(DEFAULT_US_PER_TICK, 6250, "DEFAULT_US_PER_TICK should be 6250 microseconds (6.25ms).");
    }

    #[test]
    fn test_slot_duration_constant() {
        // Verify that a slot (64 ticks) should take approximately 400ms.
        let slot_duration_ms: u64 = (DEFAULT_US_PER_TICK * DEFAULT_TICKS_PER_SLOT) / 1_000;
        assert_eq!(slot_duration_ms, 400, "A slot should be 400ms duration.");
    }

    #[test]
    fn test_epoch_constants() {
        // Verify that 1 epoch = 432000 slots.
        assert_eq!(DEFAULT_SLOTS_PER_EPOCH, 432000, "DEFAULT_SLOTS_PER_EPOCH should be 432000.");
        // Verify that 1 epoch = 2 days.
        // 1 slot = 400ms.
        // 1 day = 24 * 60 * 60 * 1000 ms = 86400000 ms.
        // 2 days = 172800000 ms.
        // 172800000 ms / 400 ms per slot = 432000 slots per epoch.
        let ms_per_slot: u64 = (DEFAULT_US_PER_TICK * DEFAULT_TICKS_PER_SLOT) / 1_000;
        let ms_per_epoch: u64 = ms_per_slot * DEFAULT_SLOTS_PER_EPOCH;
        let days_per_epoch: f64 = ms_per_epoch as f64 / (24.0 * 60.0 * 60.0 * 1_000.0);

        assert!((days_per_epoch - 2.0).abs() < 0.001, "An epoch should be approximately 2 days.");
    }
}
