#[cfg(test)]
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::constants::TICK_DURATION_US;
use crate::poh::core::{PoH, PoHRecord};
use crate::poh::thread::thread;

/// Verifies that inserting an event changes the hash of the record.
/// This test ensures that the hash of a record is different after an event is inserted.
/// Additionally, it verifies that the inserted event is present in the record and has the
/// correct data.
#[test]
fn insert_event_changes_hash() -> () {
    let seed: &[u8; 64] = &[b'0'; 64];
    let mut poh: PoH = PoH::new(seed);

    let record1: PoHRecord = poh.next_tick();
    let record2: PoHRecord = poh.insert_event(b"TestEvent");

    // The hash of record2 must be different because an event is inserted.
    assert_ne!(record1.hash, record2.hash);
    assert!(record2.event.is_some());
    assert_eq!(record2.event.unwrap(), b"TestEvent".to_vec());
}

/// Verifies that the `verify_records` function correctly verifies a records with events.
/// The test creates a records with events and verifies that the records is valid.
/// Then it modifies one event's data to test that the records is invalid after that.
#[test]
fn records_verification_with_events() -> () {
    let seed: &[u8; 64] = &[b'0'; 64];
    let mut poh: PoH = PoH::new(seed);
    let mut records: Vec<PoHRecord> = Vec::new();
    let empty_records: Vec<PoHRecord> = Vec::new();

    // Verify empty records.
    assert!(!PoH::verify_records(&empty_records));

    // Create a records with some events.
    for i in 0..20 {
        let record: PoHRecord = if i % 5 == 0 {
            poh.insert_event(format!("Event {}", i).as_bytes())
        } else {
            poh.next_tick()
        };
        records.push(record);
    }

    assert!(PoH::verify_records(&records));

    // Modify one event data to test failed verification.
    if let Some(record) = records.get_mut(10) {
        if let Some(ref mut evt) = record.event {
            evt[0] ^= 0xFF; // modify event data.
        }
    }
    assert!(!PoH::verify_records(&records));
}

/// Verifies that the timestamps in the generated PoH records are monotonically increasing
/// and within a reasonable tolerance of the expected values.
///
/// This test spawns a PoH generator thread to produce a series of PoH records with a fixed
/// seed and number of ticks. It collects the records and asserts that the timestamps are
/// monotonically increasing and do not deviate more than the allowed drift from the expected
/// timestamps, based on the tick duration.
#[test]
fn timestamp_verification() -> () {
    let seed: &[u8; 64] = &[b'0'; 64];
    let max_ticks: u64 = 20;

    let rx: Receiver<PoHRecord> = thread(seed, max_ticks);
    let mut records: Vec<PoHRecord> = Vec::new();

    while let Ok(record) = rx.recv() {
        records.push(record);
    }

    assert!(PoH::verify_timestamps(&records));
}

/// Verifies that the PoH generation thread runs at the expected speed by measuring the time
/// taken to generate a fixed number of ticks. The test also calculates some statistics about
/// the tick timing and prints them to the console. The test fails if the total duration exceeds
/// a reasonable tolerance of the expected value.
#[test]
fn time_accuracy() -> () {
    let seed: &[u8; 64] = &[b'0'; 64];
    let max_ticks: u64 = 480; // Generate 480 ticks (~7.5 slots, ~3 seconds).
    let start: Instant = Instant::now();
    let rx: Receiver<PoHRecord> = thread(seed, max_ticks);
    let mut records: Vec<PoHRecord> = Vec::new();
    let mut tick_times: Vec<Duration> = Vec::new();
    let mut last_tick: Instant = start;

    while let Ok(record) = rx.recv() {
        let now: Instant = Instant::now();
        tick_times.push(now - last_tick);
        last_tick = now;
        records.push(record);
    }

    let duration: Duration = start.elapsed();
    println!("Total duration: {:?}", duration);
    // Calculate statistics about tick timing.
    if !tick_times.is_empty() {
        let max_tick: &Duration = tick_times.iter().max().unwrap();
        let avg_tick: Duration = tick_times.iter().sum::<Duration>() / tick_times.len() as u32;
        println!("Max tick duration: {:?}", max_tick);
        println!("Avg tick duration: {:?}", avg_tick);
        // Count ticks that exceeded limit.
        let over_limit: usize = tick_times.iter().filter(|&&d| d > Duration::from_micros(TICK_DURATION_US)).count();
        println!("Ticks exceeding limit: {} of {}", over_limit, tick_times.len());
    }
    // Should be 3 seconds with a tolerance of ~3ms.
    assert!(duration < Duration::from_micros(3_003_000));
}
