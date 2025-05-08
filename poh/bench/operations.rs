use std::time::{Duration, Instant};

use poh::types::{PoH, PoHRecord};

use lib::utils::hash;
use lib::{DEFAULT_HASHES_PER_TICK, DEFAULT_US_PER_TICK};

use criterion::{BenchmarkGroup, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn bench_hash_operations(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Hash Operations");

    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    // Benchmark single hash operation.
    group.bench_function("single_hash", |b| {
        let data: [u8; 64] = [0u8; 64];
        b.iter(|| hash::hash(black_box(&data)))
    });
    // Benchmark hash with data (event insertion).
    group.bench_function("hash_with_data", |b| {
        let prev_hash: [u8; 32] = [1u8; 32];
        let data: &'static [u8; 38] = b"This is an event data for benchmarking";
        b.iter(|| hash::hash_with_data(black_box(&prev_hash), black_box(data)))
    });
    // Benchmark extending hash chain with different iteration counts.
    for iterations in [100, 1000, DEFAULT_HASHES_PER_TICK].iter() {
        group.bench_with_input(BenchmarkId::new("extend_hash_chain", iterations), iterations, |b, &iterations| {
            let prev_hash: [u8; 32] = [2u8; 32];
            b.iter(|| hash::extend_hash_chain(black_box(&prev_hash), black_box(iterations)))
        });
    }
    group.finish();
}

fn bench_poh_core(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Core Operations");

    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    // Benchmark PoH initialization.
    group.bench_function("poh_new", |b| {
        let seed: [u8; 64] = [b'0'; 64];
        b.iter(|| PoH::new(black_box(&seed)))
    });
    // Benchmark ticking.
    group.bench_function("next_tick", |b| {
        let seed: [u8; 64] = [b'0'; 64];
        let mut poh: PoH = PoH::new(&seed);
        b.iter(|| poh.next_tick())
    });
    // Benchmark event insertion.
    group.bench_function("insert_event", |b| {
        let seed: [u8; 64] = [b'0'; 64];
        let mut poh: PoH = PoH::new(&seed);
        let event_data: &'static [u8; 38] = b"This is an event for benchmark testing";
        b.iter(|| poh.insert_event(black_box(event_data)))
    });
    group.finish();
}

// Benchmark verification operations
fn bench_verification(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Verification");

    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    // Benchmark hash chain verification.
    group.bench_function("verify_hash_chain", |b| {
        let prev_hash: [u8; 32] = [3u8; 32];
        let extended: [u8; 32] = hash::extend_hash_chain(&prev_hash, DEFAULT_HASHES_PER_TICK);
        b.iter(|| hash::verify_hash_chain(black_box(&prev_hash), black_box(&extended), black_box(DEFAULT_HASHES_PER_TICK), black_box(None)))
    });
    // Benchmark hash chain verification with event data.
    group.bench_function("verify_hash_chain_with_event", |b| {
        let prev_hash: [u8; 32] = [4u8; 32];
        let event_data: &'static [u8; 37] = b"Event data for verification benchmark";
        let mut hash: [u8; 32] = hash::hash_with_data(&prev_hash, event_data);

        hash = hash::extend_hash_chain(&hash, DEFAULT_HASHES_PER_TICK);
        b.iter(|| {
            hash::verify_hash_chain(
                black_box(&prev_hash),
                black_box(&hash),
                black_box(DEFAULT_HASHES_PER_TICK),
                black_box(Some(event_data)),
            )
        })
    });
    group.finish();
}

fn bench_poh_generation(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("PoH Generation");
    group.warm_up_time(Duration::from_millis(1000));
    group.measurement_time(Duration::from_secs(5));
    // Generate a sequence of ticks.
    for tick_count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("generate_ticks", tick_count), tick_count, |b, &tick_count| {
            b.iter(|| {
                let seed: [u8; 64] = [b'0'; 64];
                let mut poh: PoH = PoH::new(&seed);
                let mut records: Vec<PoHRecord> = Vec::with_capacity(tick_count as usize);

                for i in 0..tick_count {
                    let record: PoHRecord = if i % 10 == 0 {
                        // Every 10th tick, insert an event.
                        let event_data = format!("Event at tick {}", i);
                        poh.insert_event(event_data.as_bytes())
                    } else {
                        poh.next_tick()
                    };
                    records.push(record);
                }
                black_box(records)
            })
        });
    }
    group.finish();
}

// Benchmark SHA256 vs BLAKE3 hash algorithms
fn bench_hash_algorithms(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Hash Algorithms");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    // Test data with different sizes.
    let test_data_small: Vec<u8> = vec![0u8; 64];
    let test_data_medium: Vec<u8> = vec![0u8; 1024];
    let test_data_large: Vec<u8> = vec![0u8; 1024 * 1024]; // 1MB.

    // First benchmark SHA256 (algorithm 0).
    hash::set_hash_algorithm(0);

    for (name, data) in [
        ("SHA256_small", &test_data_small),
        ("SHA256_medium", &test_data_medium),
        ("SHA256_large", &test_data_large),
    ]
    .iter()
    {
        group.bench_function(*name, |b| b.iter(|| hash::hash(black_box(data))));
    }

    // Then benchmark BLAKE3 (algorithm 1).
    hash::set_hash_algorithm(1);

    for (name, data) in [
        ("BLAKE3_small", &test_data_small),
        ("BLAKE3_medium", &test_data_medium),
        ("BLAKE3_large", &test_data_large),
    ]
    .iter()
    {
        group.bench_function(*name, |b| b.iter(|| hash::hash(black_box(data))));
    }
    // Reset to default algorithm.
    hash::set_hash_algorithm(0);
    group.finish();
}

fn bench_realtime_performance(c: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, criterion::measurement::WallTime> = c.benchmark_group("Real-time Performance");
    group.warm_up_time(Duration::from_millis(500));
    // Use shorter measurement time for real-time tests.
    group.measurement_time(Duration::from_secs(2));
    // Benchmark time to perform one complete tick cycle.
    group.bench_function("tick_cycle_time", |b| {
        b.iter_custom(|iters| {
            let mut total_duration: Duration = Duration::new(0, 0);
            let seed: [u8; 64] = [b'0'; 64];

            for _ in 0..iters {
                let mut poh: PoH = PoH::new(&seed);
                let start: Instant = Instant::now();
                // Generate a tick with precise timing.
                let next_tick_target_us = DEFAULT_US_PER_TICK;
                let record: PoHRecord = poh.next_tick();
                // Simulate waiting for next tick.
                let elapsed_us: u64 = start.elapsed().as_micros() as u64;

                if elapsed_us < next_tick_target_us {
                    let sleep_us: u64 = next_tick_target_us - elapsed_us;
                    std::thread::sleep(Duration::from_micros(sleep_us));
                }

                total_duration += start.elapsed();
                black_box(record);
            }
            total_duration
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_hash_operations,
    bench_poh_core,
    bench_verification,
    bench_poh_generation,
    bench_hash_algorithms,
    bench_realtime_performance,
);
criterion_main!(benches);
