use std::time::{Duration, Instant};

use super::core::{PoH, PoHRecord};

#[cfg(test)]
mod tests {
    use super::*;
    /// Verifies that inserting an event changes the hash of the record.
    /// This test ensures that the hash of a record is different after an event is inserted.
    /// Additionally, it verifies that the inserted event is present in the record and has the
    /// correct data.
    #[test]
    fn test_insert_event_changes_hash() -> () {
        let seed: &[u8; 9] = b"EventSeed";
        let mut poh: PoH = PoH::new(seed);

        let record1: PoHRecord = poh.next_tick();
        let record2: PoHRecord = poh.insert_event(b"TestEvent");

        // The hash of record2 must be different because an event is inserted.
        assert_ne!(record1.hash, record2.hash);
        assert!(record2.event.is_some());
        assert_eq!(record2.event.unwrap(), b"TestEvent".to_vec());
    }

    /// Verifies that the `verify_chain` function correctly verifies a chain with events.
    /// The test creates a chain with events and verifies that the chain is valid.
    /// Then it modifies one event's data to test that the chain is invalid after that.
    #[test]
    fn test_chain_verification_with_events() -> () {
        let seed: &[u8; 15] = b"VerifyEventSeed";
        let mut poh: PoH = PoH::new(seed);
        let mut chain: Vec<PoHRecord> = Vec::new();

        // Create a chain with some events.
        for i in 0..20 {
            let record: PoHRecord = if i % 5 == 0 {
                poh.insert_event(format!("Event {}", i).as_bytes())
            } else {
                poh.next_tick()
            };
            chain.push(record);
        }

        assert!(PoH::verify_chain(&chain));

        // Modify one event data to test failed verification.
        if let Some(record) = chain.get_mut(10) {
            if let Some(ref mut evt) = record.event {
                evt[0] ^= 0xFF; // modify event data.
            }
        }
        assert!(!PoH::verify_chain(&chain));
    }

    /// Verifies the performance of the Proof of History generator.
    /// This test measures the time it takes to generate 1000 ticks of the PoH chain.
    /// The test should finish in much less than 1 second.
    #[test]
    fn test_performance() -> () {
        let seed: &[u8; 64] = &[b'0'; 64];
        let mut poh: PoH = PoH::new(seed);
        let start: Instant = Instant::now();

        for _ in 0..1000 {
            let _ = poh.next_tick();
        }

        let duration: Duration = start.elapsed();

        println!("1000 ticks generation time: {:?}", duration);
        assert!(duration < Duration::from_secs(1)); // Should be much less than 1 second.
    }
}
