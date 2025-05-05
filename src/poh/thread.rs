use std::sync::mpsc::{Receiver, SendError, SyncSender, sync_channel};
use std::thread;
use std::time::{Duration, Instant};

use crate::poh::core::{PoH, PoHRecord};
use crate::poh::hash;
use crate::{BATCH_SIZE, CHANNEL_CAPACITY, SPINLOCK_THRESHOLD_US, TICK_DURATION_US};

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
pub fn thread(seed: &[u8], max_ticks: u64) -> Receiver<PoHRecord> {
    let (tx, rx) = sync_channel(CHANNEL_CAPACITY);
    let seed: Vec<u8> = seed.to_vec();

    thread::spawn(move || {
        let mut poh: PoH = PoH::new(&seed);
        let mut records_batch: Vec<PoHRecord> = Vec::with_capacity(BATCH_SIZE);

        let start: Instant = Instant::now();
        // Pre-calculate target completion times for each tick.
        let mut next_tick_target_us: u64 = TICK_DURATION_US;

        // Use a higher priority thread if possible.
        #[cfg(target_os = "linux")]
        {
            use std::io;
            use std::thread::ThreadId;
            // Attempt to set thread to real-time priority (requires root privileges).
            // This is optional and will silently fail if not running with proper permissions.
            if let Some(thread_id) = std::thread::current().id().as_u64() {
                let _ = unsafe {
                    libc::pthread_setschedprio(thread_id as usize as *mut libc::c_void, libc::SCHED_FIFO);
                };
            }
        }

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
            if records_batch.len() >= BATCH_SIZE && send_batch(&tx, &mut records_batch).is_err() {
                break;
            }

            let elapsed_us: u64 = start.elapsed().as_micros() as u64;
            let target_us: u64 = next_tick_target_us;

            if elapsed_us < target_us {
                let sleep_us: u64 = target_us - elapsed_us;
                // Use spin waiting for very short sleeps to improve precision.
                if sleep_us < SPINLOCK_THRESHOLD_US {
                    // Spin wait for greater timing precision.
                    let spin_until: u64 = start.elapsed().as_micros() as u64 + sleep_us;
                    while start.elapsed().as_micros() < spin_until as u128 {
                        // Insert a pause instruction to reduce CPU usage during spin-waiting.
                        #[cfg(target_arch = "x86_64")]
                        unsafe {
                            std::arch::x86_64::_mm_pause();
                        }
                    }
                } else {
                    // Use normal sleep for longer durations.
                    thread::sleep(Duration::from_micros(sleep_us));
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
fn send_batch(tx: &SyncSender<PoHRecord>, batch: &mut Vec<PoHRecord>) -> Result<(), SendError<PoHRecord>> {
    for record in batch.drain(..) {
        tx.send(record)?;
    }
    Ok(())
}

/// Compute a specific number of hashes to simulate CPU-intensive PoH generation
///
/// This function is used as part of the PoH process to ensure that the proper
/// number of hashes (work) is done for each tick
#[inline]
pub fn compute_hashes(iterations: u64) {
    // Use a zero-initialized hash as starting point.
    let zero_hash = [0u8; 32];
    // Use our centralized hash function to extend the hash chain.
    let _ = hash::extend_hash_chain(&zero_hash, iterations);
}
