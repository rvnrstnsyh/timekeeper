use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use crate::poh::constants::{BATCH_SIZE, TICK_DURATION_US};
use crate::poh::core::{PoH, PoHRecord};

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
        // Pre-calculate target completion times for each tick
        let mut next_tick_target_us: u64 = TICK_DURATION_US;

        for i in 0..max_ticks {
            // Simulate event insertion every 10 ticks.
            let record: PoHRecord = if i % 10 == 0 {
                let event_data: String = format!("Event at tick {}", i);
                poh.insert_event(event_data.as_bytes())
            } else {
                poh.next_tick()
            };

            records_batch.push(record);

            // Send in batches but don't let batch operations delay timing.
            if records_batch.len() >= BATCH_SIZE {
                if send_batch(&tx, &mut records_batch).is_err() {
                    break;
                }
            }

            // More precise timing control.
            let elapsed_us: u64 = start.elapsed().as_micros() as u64;
            let target_us: u64 = next_tick_target_us;

            if elapsed_us < target_us {
                // Use spin wait for sub-millisecond precision instead of sleep for very short waits.
                if target_us - elapsed_us < 500 {
                    while start.elapsed().as_micros() < target_us as u128 {}
                } else {
                    thread::sleep(Duration::from_micros(target_us - elapsed_us));
                }
            }
            // Calculate next tick target time.
            next_tick_target_us += TICK_DURATION_US;
        }
        // Send any remaining records.
        let _ = send_batch(&tx, &mut records_batch);
    });
    return rx;
}

/// Sends a batch of `PoHRecord` items through a synchronous channel.
///
/// This function drains the provided vector of `PoHRecord` items and sends each record
/// through the given `SyncSender`. Once all records are sent, the batch vector is empty.
///
/// # Parameters
/// - `tx`: A reference to a synchronous sender used to send `PoHRecord` items.
/// - `batch`: A mutable reference to a vector of `PoHRecord` items to be sent. The vector
///   will be empty after this function is executed.
///
/// # Returns
/// A `Result` indicating success or failure. If sending a record fails, an `mpsc::SendError`
/// containing the unsent record is returned.
fn send_batch(
    tx: &SyncSender<PoHRecord>,
    batch: &mut Vec<PoHRecord>,
) -> Result<(), mpsc::SendError<PoHRecord>> {
    for record in batch.drain(..) {
        tx.send(record)?;
    }
    Ok(())
}
