use std::thread;

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};

use super::constants::{BATCH_SIZE, TICK_DURATION_US};
use super::core::{PoH, PoHRecord};

/// Spawns a thread that generates a Proof of History (PoH) chain with the given seed and
/// maximum number of ticks. The thread sends `PoHRecord`s over a channel for consumption.
///
/// The thread simulates event insertion every 10 ticks and sends `PoHRecord`s in batches
/// for better performance. The sleep time is calculated more accurately by accounting for
/// processing time.
///
/// # Parameters
/// - `seed`: A byte slice used to initialize the PoH chain.
/// - `max_ticks`: The maximum number of ticks to generate.
///
/// # Returns
/// A `Receiver` that can be used to receive `PoHRecord`s from the spawned thread.
pub fn poh_thread(seed: &[u8], max_ticks: u64) -> Receiver<PoHRecord> {
    const CHANNEL_CAPACITY: usize = 100; // Buffer size to prevent blocking.

    let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
    let seed: Vec<u8> = seed.to_vec();

    thread::spawn(move || {
        let mut poh: PoH = PoH::new(&seed);
        let mut records_batch: Vec<PoHRecord> = Vec::with_capacity(BATCH_SIZE);
        let start: Instant = Instant::now();

        for i in 0..max_ticks {
            // Simulate event insertion every 10 ticks.
            let record: PoHRecord = if i % 10 == 0 {
                let event_data: String = format!("Event at tick {}", i);
                poh.insert_event(event_data.as_bytes())
            } else {
                poh.next_tick()
            };

            records_batch.push(record);

            // Send in batches for better performance.
            if records_batch.len() >= BATCH_SIZE {
                if send_batch(&tx, &mut records_batch).is_err() {
                    break;
                }
            }

            // Calculate sleep time more accurately by accounting for processing time.
            let elapsed_us: u64 = start.elapsed().as_micros() as u64;
            let expected_time_us: u64 = (i + 1) * TICK_DURATION_US;

            if expected_time_us > elapsed_us {
                thread::sleep(Duration::from_micros(expected_time_us - elapsed_us));
            }
        }

        // Send any remaining records.
        let _ = send_batch(&tx, &mut records_batch);
    });

    return rx;
}

/// Sends all records in the given batch over the given channel.
///
/// # Parameters
/// - `tx`: The channel to send records over.
/// - `batch`: The batch of records to send.
///
/// # Returns
/// `Ok(())` if all records were sent successfully, `Err(())` if not.
pub fn send_batch(tx: &SyncSender<PoHRecord>, batch: &mut Vec<PoHRecord>) -> Result<(), ()> {
    for record in batch.drain(..) {
        if tx.send(record).is_err() {
            return Err(());
        }
    }
    return Ok(());
}
