use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use timekeeper::poh::core::{PoH, PoHRecord};
use timekeeper::poh::thread::{compute_hashes, thread as poh_thread};
use timekeeper::{DEFAULT_CHANNEL_CAPACITY, DEFAULT_HASHES_PER_TICK, DEFAULT_US_PER_TICK};

use criterion::{BatchSize, BenchmarkGroup, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn bench_poh_thread(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Thread Operations");

    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(5));
    // Benchmark thread spawning and initial communication.
    group.bench_function("thread_startup", |b| {
        b.iter(|| {
            let seed: [u8; 64] = [b'0'; 64];
            let max_ticks: u64 = 10; // Small number for quick startup test.
            let rx: Receiver<PoHRecord> = poh_thread(black_box(&seed), black_box(max_ticks));
            // Get first record to ensure thread is running.
            let record: PoHRecord = rx.recv().unwrap();
            black_box(record)
        })
    });

    // Benchmark thread throughput with different tick counts.
    for &tick_count in &[100, 500, 1000] {
        group.bench_with_input(BenchmarkId::new("thread_throughput", tick_count), &tick_count, |b, &tick_count| {
            b.iter_batched(
                || {
                    let seed: [u8; 64] = [b'0'; 64];
                    poh_thread(&seed, tick_count)
                },
                |rx| {
                    let mut count: i32 = 0;
                    while let Ok(_record) = rx.recv() {
                        count += 1;
                    }
                    black_box(count)
                },
                BatchSize::SmallInput,
            )
        });
    }

    // Benchmark compute_hashes function with different iteration counts.
    for &iterations in &[1000, 10000, DEFAULT_HASHES_PER_TICK] {
        group.bench_with_input(BenchmarkId::new("compute_hashes", iterations), &iterations, |b, &iterations| {
            b.iter(|| compute_hashes(black_box(iterations)))
        });
    }
    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Batch Processing");

    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    // Benchmark sending records in batches of different sizes.
    for &batch_size in &[1, 16, 64, 256] {
        group.bench_with_input(BenchmarkId::new("batch_send", batch_size), &batch_size, |b, &batch_size| {
            b.iter_batched(
                || {
                    // Setup phase: create records and channel.
                    let (tx, rx) = sync_channel::<PoHRecord>(DEFAULT_CHANNEL_CAPACITY);
                    let seed: [u8; 64] = [b'0'; 64];
                    let mut poh: PoH = PoH::new(&seed);
                    let mut records: Vec<PoHRecord> = Vec::with_capacity(batch_size as usize);

                    for i in 0..batch_size {
                        let record: PoHRecord = if i % 10 == 0 {
                            let event_data: String = format!("Event at {}", i);
                            poh.insert_event(event_data.as_bytes())
                        } else {
                            poh.next_tick()
                        };
                        records.push(record);
                    }
                    (tx, rx, records)
                },
                |(tx, rx, mut records)| {
                    // Benchmark phase: send all records.
                    for record in records.drain(..) {
                        tx.send(record).unwrap();
                    }
                    // Receive all records to avoid channel backpressure.
                    let mut received: Vec<PoHRecord> = Vec::with_capacity(batch_size as usize);
                    while let Ok(record) = rx.try_recv() {
                        received.push(record);
                        if received.len() >= batch_size as usize {
                            break;
                        }
                    }
                    black_box(received)
                },
                BatchSize::SmallInput,
            )
        });
    }

    // Benchmark channel performance with different capacities
    for &capacity in &[64, 256, 1024, DEFAULT_CHANNEL_CAPACITY] {
        group.bench_with_input(BenchmarkId::new("channel_capacity", capacity), &capacity, |b, &capacity| {
            b.iter_batched(
                || {
                    // Setup: create channel with specific capacity.
                    let (tx, rx) = sync_channel::<PoHRecord>(capacity);
                    let seed: [u8; 64] = [b'0'; 64];
                    let mut poh: PoH = PoH::new(&seed);
                    // Generate test records (fixed number for all capacities).
                    let test_records: Vec<PoHRecord> = (0..100)
                        .map(|i| {
                            if i % 10 == 0 {
                                let event_data: String = format!("Event at {}", i);
                                poh.insert_event(event_data.as_bytes())
                            } else {
                                poh.next_tick()
                            }
                        })
                        .collect::<Vec<_>>();
                    (tx, rx, test_records)
                },
                |(tx, rx, records)| {
                    // Benchmark: test producer-consumer throughput.
                    let producer: JoinHandle<()> = thread::spawn(move || {
                        for record in records {
                            if tx.send(record).is_err() {
                                break;
                            }
                        }
                    });

                    let mut received = 0;
                    while let Ok(_) = rx.recv_timeout(Duration::from_millis(100)) {
                        received += 1;
                        if received >= 100 {
                            break;
                        }
                    }
                    producer.join().unwrap();
                    black_box(received)
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_timing_precision(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Timing Precision");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    // Benchmark sleep precision for different durations.
    for &us in &[100, 250, 1000, DEFAULT_US_PER_TICK] {
        group.bench_with_input(BenchmarkId::new("sleep_precision_us", us), &us, |b, &us| {
            b.iter_custom(|iters| {
                let mut total_error: Duration = Duration::new(0, 0);
                for _ in 0..iters {
                    let start: Instant = Instant::now();
                    thread::sleep(Duration::from_micros(us));
                    let elapsed: Duration = start.elapsed();
                    // Calculate error (absolute difference from target).
                    let target: Duration = Duration::from_micros(us);
                    let error: Duration = if elapsed > target { elapsed - target } else { target - elapsed };
                    total_error += error;
                }
                total_error
            })
        });
    }

    for &us in &[50, 100, 250] {
        group.bench_with_input(BenchmarkId::new("spin_wait_precision_us", us), &us, |b, &us| {
            b.iter_custom(|iters| {
                let mut total_error: Duration = Duration::new(0, 0);

                for _ in 0..iters {
                    let start: Instant = Instant::now();
                    let spin_until: Instant = start + Duration::from_micros(us);

                    // Spin wait.
                    while Instant::now() < spin_until {
                        // CPU-friendly pause.
                        #[cfg(target_arch = "x86_64")]
                        unsafe {
                            std::arch::x86_64::_mm_pause();
                        }
                    }

                    let elapsed: Duration = start.elapsed();
                    let target: Duration = Duration::from_micros(us);
                    let error: Duration = if elapsed > target { elapsed - target } else { target - elapsed };

                    total_error += error;
                }
                total_error
            })
        });
    }
    group.finish();
}

criterion_group!(thread_benches, bench_poh_thread, bench_batch_processing, bench_timing_precision,);
criterion_main!(thread_benches);
