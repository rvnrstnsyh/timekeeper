use std::sync::mpsc::{Receiver, SendError, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread as std_thread;
use std::time::{Duration, Instant};

use crate::types::{PoH, PoHRecord};

use lib::{DEFAULT_BATCH_SIZE, DEFAULT_CHANNEL_CAPACITY, DEFAULT_SPINLOCK_THRESHOLD_US, DEFAULT_US_PER_TICK};
use thread::native::types::{Config, JoinHandle, Manager, SharedJoinHandle, ThreadHandleGuard, ThreadHandlePool};

use anyhow::Result;

lazy_static::lazy_static! {
    static ref ACTIVE_THREADS: ThreadHandlePool = Mutex::new(Vec::new());
}

pub fn thread(seed: &[u8], max_ticks: u64) -> Result<Receiver<PoHRecord>> {
    let (tx, rx) = sync_channel(DEFAULT_CHANNEL_CAPACITY);
    let seed: Vec<u8> = seed.to_vec();

    let manager: Manager = Manager::new("poh-thread".to_string(), Config::default())?;
    let handle: SharedJoinHandle = Arc::new(Mutex::new(None));

    let instance: JoinHandle<()> = manager
        .spawn(move || {
            let mut poh: PoH = PoH::new(&seed);
            let mut records_batch: Vec<PoHRecord> = Vec::with_capacity(DEFAULT_BATCH_SIZE);

            let start: Instant = Instant::now();
            // Pre-calculate target completion times for each tick.
            let mut next_tick_target_us: u64 = DEFAULT_US_PER_TICK;

            for i in 0..max_ticks {
                // Simulate event insertion every 10 ticks.
                let record: PoHRecord = if i % 10 == 0 {
                    let event_data: String = format!("Event at tick {}.", i);
                    poh.insert_event(event_data.as_bytes())
                } else {
                    poh.next_tick()
                };

                records_batch.push(record);
                // Send in batches but don't let batch operations delay timing.
                if records_batch.len() >= DEFAULT_BATCH_SIZE && send_batch(&tx, &mut records_batch).is_err() {
                    break;
                }

                let elapsed_us: u64 = start.elapsed().as_micros() as u64;
                let target_us: u64 = next_tick_target_us;

                if elapsed_us < target_us {
                    let sleep_us: u64 = target_us.saturating_sub(elapsed_us);
                    // Use spin waiting for very short sleeps to improve precision.
                    if sleep_us < DEFAULT_SPINLOCK_THRESHOLD_US {
                        // Spin wait for greater timing precision.
                        let spin_until: u128 = start.elapsed().as_micros().saturating_add(sleep_us as u128);
                        while start.elapsed().as_micros() < spin_until {
                            // Insert a pause instruction to reduce CPU usage during spin-waiting.
                            #[cfg(target_arch = "x86_64")]
                            unsafe {
                                std::arch::x86_64::_mm_pause();
                            }
                        }
                    } else {
                        // Use normal sleep for longer durations.
                        std_thread::sleep(Duration::from_micros(sleep_us));
                    }
                }
                // Calculate next tick target time.
                next_tick_target_us = next_tick_target_us.saturating_add(DEFAULT_US_PER_TICK);
            }
            // Send any remaining records.
            let _ = send_batch(&tx, &mut records_batch);
        })
        .expect("Failed to spawn PoH thread.");
    // Store the instance.
    *handle.lock().unwrap() = Some(instance);
    ACTIVE_THREADS.lock().unwrap().push(handle);

    return Ok(rx);
}

fn send_batch(tx: &SyncSender<PoHRecord>, batch: &mut Vec<PoHRecord>) -> Result<(), SendError<PoHRecord>> {
    for record in batch.drain(..) {
        tx.send(record)?;
    }
    return Ok(());
}

pub fn cleanup_threads() {
    let mut handles: ThreadHandleGuard = ACTIVE_THREADS.lock().unwrap();

    for handle_arc in handles.drain(..) {
        if let Some(join_handle) = handle_arc.lock().unwrap().take() {
            let _ = join_handle.join();
        }
    }
}
