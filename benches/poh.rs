use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use timekeeper::poh::core::{PoH, PoHRecord};
use timekeeper::poh::hash;
use timekeeper::poh::thread::{compute_hashes, thread};
use timekeeper::{HASHES_PER_TICK, SLOTS_PER_EPOCH, TICK_DURATION_US, TICKS_PER_SLOT};

// Benchmark the core hash function performance.
fn benchmark_hash_function(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Hash Functions");

    // Benchmark single hash operation.
    group.bench_function("single_hash", |b| {
        let data: &[u8; 14] = b"benchmark data";
        b.iter(|| hash::hash(data))
    });
    // Benchmark hash with data.
    group.bench_function("hash_with_data", |b| {
        let prev_hash: [u8; 32] = [0u8; 32];
        let data: &[u8; 10] = b"event data";
        b.iter(|| hash::hash_with_data(&prev_hash, data))
    });
    // Benchmark hash chain extension with different iteration counts.
    for iterations in [100, 1000, HASHES_PER_TICK].iter() {
        group.bench_with_input(BenchmarkId::new("extend_hash_chain", iterations), iterations, |b, &iterations| {
            let prev_hash: [u8; 32] = [0u8; 32];
            b.iter(|| hash::extend_hash_chain(&prev_hash, iterations))
        });
    }
    group.finish();
}

// Benchmark the compute_hashes function specifically.
fn benchmark_compute_hashes(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Computation");

    for iterations in [1000, 5000, HASHES_PER_TICK].iter() {
        group.bench_with_input(BenchmarkId::new("compute_hashes", iterations), iterations, |b, &iterations| {
            b.iter(|| compute_hashes(iterations))
        });
    }
    group.finish();
}

// Benchmark PoH core functionality.
fn benchmark_poh_core(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Core");

    group.bench_function("next_tick", |b| {
        let seed: [u8; 64] = [b'0'; 64];
        let mut poh: PoH = PoH::new(&seed);
        b.iter(|| poh.next_tick())
    });
    group.bench_function("insert_event", |b| {
        let seed: [u8; 64] = [b'0'; 64];
        let mut poh: PoH = PoH::new(&seed);
        let event_data: &[u8; 20] = b"benchmark event data";
        b.iter(|| poh.insert_event(event_data))
    });
    group.finish();
}

// Benchmark the ability to maintain the target tick rate.
fn benchmark_tick_rate(c: &mut Criterion) {
    // Create a custom benchmark group with specific measurement settings.
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Tick Rate");

    // Configure the benchmark for a longer duration to get more accurate measurements.
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    // Benchmark tick rate over different numbers of ticks.
    for num_ticks in [100, 200, 400].iter() {
        group.bench_with_input(BenchmarkId::new("sequential_ticks", num_ticks), num_ticks, |b, &num_ticks| {
            b.iter(|| {
                // Measure how long it takes to generate the specified number of ticks.
                let seed: [u8; 64] = [b'0'; 64];
                let rx: Receiver<PoHRecord> = thread(&seed, num_ticks);

                // Collect records and measure timing accuracy.
                let start: Instant = Instant::now();
                let mut records_count: usize = 0;
                while let Ok(_) = rx.recv() {
                    records_count += 1;
                }

                let duration: Duration = start.elapsed();
                assert_eq!(records_count, num_ticks as usize);

                // Expected duration based on tick rate specification.
                let expected_duration_us: u64 = num_ticks * TICK_DURATION_US;
                let actual_duration_us: u64 = duration.as_micros() as u64;
                // Calculate deviation percentage.
                let deviation: f64 = if actual_duration_us > expected_duration_us {
                    actual_duration_us as f64 / expected_duration_us as f64 - 1.0
                } else {
                    1.0 - actual_duration_us as f64 / expected_duration_us as f64
                };

                // Verify that we're within tolerance.
                assert!(
                    deviation < 0.15,
                    "Timing deviation exceeds 15%: expected {}μs, got {}μs, deviation: {:.2}%",
                    expected_duration_us,
                    actual_duration_us,
                    deviation * 100.0
                );
                // Return something to satisfy the benchmark.
                return records_count;
            })
        });
    }
    group.finish();
}

// Test throughput capability - how many ticks can be generated per second.
fn benchmark_throughput(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Throughput");

    // Use a "no_wait" version of the thread function for throughput testing.
    // This will run as fast as possible without waiting for timing.
    group.bench_function("max_ticks_per_second", |b| {
        b.iter_custom(|iters| {
            let seed: [u8; 64] = [b'0'; 64];
            let mut poh: PoH = PoH::new(&seed);

            let start = Instant::now();
            for _ in 0..iters {
                // Generate ticks as fast as possible.
                let _ = poh.next_tick();
            }
            start.elapsed()
        })
    });

    group.finish();
}

// Test verification performance.
fn benchmark_verification(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Verification");
    // Generate a set of records to verify.
    let seed: [u8; 64] = [b'0'; 64];
    let mut poh: PoH = PoH::new(&seed);
    let mut records: Vec<PoHRecord> = Vec::with_capacity(100);

    for i in 0..100 {
        if i % 10 == 0 {
            records.push(poh.insert_event(format!("Event {}", i).as_bytes()));
        } else {
            records.push(poh.next_tick());
        }
    }

    // Benchmark record verification.
    group.bench_function("verify_records", |b| b.iter(|| PoH::verify_records(&records)));
    // Benchmark timestamp verification.
    group.bench_function("verify_timestamps", |b| b.iter(|| PoH::verify_timestamps(&records)));
    group.finish();
}

// Test device capability against requirements.
fn benchmark_system_capability(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("System Capability");

    group.measurement_time(Duration::from_secs(30));
    group.sample_size(20);

    // Calculate key metrics based on requirements.
    let ticks_per_second: u64 = 1_000_000 / TICK_DURATION_US;
    let hashes_per_second: u64 = ticks_per_second * HASHES_PER_TICK;
    let slots_per_second: f64 = ticks_per_second as f64 / TICKS_PER_SLOT as f64;
    let slots_per_epoch: u64 = SLOTS_PER_EPOCH;
    let epoch_duration_seconds: u64 = (slots_per_epoch as f64 / slots_per_second) as u64;

    println!("Target PoH specifications:");
    println!("- Hashes per tick: {}", HASHES_PER_TICK);
    println!("- Tick duration: {:.2}ms", TICK_DURATION_US as f64 / 1000.0);
    println!("- Ticks per slot: {}", TICKS_PER_SLOT);
    println!("- Slots per epoch: {}", SLOTS_PER_EPOCH);
    println!("- Target throughput: {} ticks/second", ticks_per_second);
    println!("- Target hash rate: {:.3} MH/s", hashes_per_second as f64 / 1_000_000.0);
    println!("- Target epoch duration: {} days", epoch_duration_seconds / (24 * 60 * 60));

    // Test timing consistency over multiple slots (each slot has 64 ticks).
    let num_slots_to_test: u64 = 5;
    let num_ticks: u64 = num_slots_to_test * TICKS_PER_SLOT;

    group.bench_function("multi_slot_timing", |b| {
        b.iter(|| {
            let seed: [u8; 64] = [b'0'; 64];
            let rx: Receiver<PoHRecord> = thread(&seed, num_ticks);

            let start: Instant = Instant::now();
            let mut records: Vec<PoHRecord> = Vec::with_capacity(num_ticks as usize);

            while let Ok(record) = rx.recv() {
                records.push(record);
            }

            let duration: Duration = start.elapsed();
            // Calculate expected slot duration.
            let _expected_slot_duration_us: u64 = TICKS_PER_SLOT * TICK_DURATION_US;
            let expected_total_duration_us: u64 = num_ticks * TICK_DURATION_US;
            let actual_duration_us: u64 = duration.as_micros() as u64;
            // Calculate deviation.
            let deviation: f64 = (actual_duration_us as f64 - expected_total_duration_us as f64) / expected_total_duration_us as f64;

            // Check if we're meeting the timing requirements.
            assert!(
                deviation.abs() < 0.15,
                "Slot timing deviation exceeds tolerance: expected {}μs, got {}μs, deviation: {:.2}%",
                expected_total_duration_us,
                actual_duration_us,
                deviation * 100.0
            );
            // Verify timestamps are correct.
            assert!(PoH::verify_timestamps(&records), "Timestamp verification failed");
            // Return the number of records for the benchmark.
            return records.len();
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_hash_function,
    benchmark_compute_hashes,
    benchmark_poh_core,
    benchmark_tick_rate,
    benchmark_throughput,
    benchmark_verification,
    benchmark_system_capability,
);
criterion_main!(benches);
