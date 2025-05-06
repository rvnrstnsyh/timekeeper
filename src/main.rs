use std::env;
use std::io::stdout;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use timekeeper::helpers::args::{OutputType, parse_args, print_usage};
use timekeeper::helpers::io::save_poh_records_to_json;
use timekeeper::poh::core::PoHRecord;
use timekeeper::poh::thread::thread;
use timekeeper::{DEFAULT_HASHES_PER_TICK, DEFAULT_SLOTS_PER_EPOCH, DEFAULT_TICKS_PER_SLOT};

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

/// Runs a Proof of History (PoH) generator with event insertion and prints the PoH records to stdout.
/// The PoH generator is started with a given seed and maximum number of ticks, and records are received
/// over a channel and printed to stdout until the generator finishes. The number of records received is
/// printed after the generator finishes.
fn main() {
    // Process command line arguments.
    let args: Vec<String> = env::args().collect();
    let output_type: OutputType = match parse_args(&args) {
        Ok(out) => out,
        Err(msg) => {
            eprintln!("{}", msg);
            print_usage();
            return;
        }
    };

    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();

    // Default seed - 64 bytes of '0'.
    let seed: [u8; 64] = [b'0'; 64];
    // Generate enough ticks for 5 minutes (about 48,000 ticks at 6.25ms per tick).
    // 5 minutes = 300 seconds.
    // 300 seconds / (6.25 / 1000) seconds per tick = 48,000 ticks.
    let target_ticks: u64 = 48_000;

    let hashes_approx: u64 = target_ticks * DEFAULT_HASHES_PER_TICK;
    let slots_approx: u64 = target_ticks / DEFAULT_TICKS_PER_SLOT;
    let duration_approx: f64 = target_ticks as f64 * 6.25 / 1000.0;

    println!("Fixed Values:                    Testing Targets:");
    println!("  | 1 Tick equals 12500 hashes     | Terminated at {} ticks ({} hashes)", target_ticks, hashes_approx);
    println!("  | 1 Tick should be 6.25ms        | Approximate {} slots", slots_approx);
    println!("  | 1 Slot is 64 Ticks             | Approximate duration is {} seconds\n  |", duration_approx);

    let start_time: Instant = Instant::now();
    let rx: Receiver<PoHRecord> = thread(&seed, target_ticks);
    let mut records_received: u64 = 0;

    // Performance tracking.
    let mut last_update: Instant = Instant::now();
    let mut last_tick_count: u64 = 0;

    // Pre-allocate vector with expected capacity if writing to JSON.
    let mut all_records: Vec<PoHRecord> = match output_type {
        OutputType::JsonFile(_) => Vec::with_capacity(target_ticks as usize),
        _ => Vec::new(),
    };

    // Common progress display code.
    let update_progress = |records: u64, tick_count: u64, last_time: Instant| -> (u64, Instant) {
        let now: Instant = Instant::now();
        let elapsed: f64 = now.duration_since(last_time).as_secs_f64();

        if elapsed >= 1.0 {
            let ticks: u64 = records - tick_count;
            let tick_per_second: f64 = ticks as f64 / elapsed;
            let hash_per_second: f64 = tick_per_second * DEFAULT_HASHES_PER_TICK as f64;
            let progress_percent: f64 = (records as f64 / target_ticks as f64) * 100.0;

            execute!(
                stdout(),
                Clear(ClearType::CurrentLine),
                MoveTo(0, 6),
                SetForegroundColor(Color::Green),
                Print(format!(
                    "  | {:.1}% - {:.2} ticks/s - {:.3} Mh/s\n",
                    progress_percent,
                    tick_per_second,
                    hash_per_second / 1_000_000.0
                )),
                ResetColor
            )
            .unwrap();

            return (records, now);
        }
        return (tick_count, last_time);
    };

    execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();

    let counting_since: u64 = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
    let datetime: DateTime<Utc> = DateTime::<Utc>::from_timestamp(counting_since as i64, 0).expect("Invalid timestamp");
    let formatted_time: String = datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    // Standardized output message for both modes.
    println!("Proof of History (PoH) has been counting since {} (Unix epoch time)", formatted_time);
    execute!(stdout(), ResetColor).unwrap();
    // Process incoming records.
    while let Ok(record) = rx.recv() {
        if let OutputType::JsonFile(_) = &output_type {
            all_records.push(record.clone())
        }
        // Show progress.
        let result: (u64, Instant) = update_progress(records_received, last_tick_count, last_update);
        last_tick_count = result.0;
        last_update = result.1;
        execute!(stdout(), MoveTo(0, 7), Clear(ClearType::CurrentLine), Print(format!("  |\n{}\n", record))).unwrap();
        records_received += 1;
    }
    // Save to JSON if needed.
    if let OutputType::JsonFile(filename) = output_type {
        match save_poh_records_to_json(&all_records, &filename) {
            Ok(_) => println!("Successfully saved {} records to file {}.", records_received, filename),
            Err(e) => eprintln!("Error saving file: {}", e),
        }
    }

    let duration: Duration = start_time.elapsed();
    let seconds: f64 = duration.as_secs_f64();
    let ticks_per_second: f64 = records_received as f64 / seconds;
    let ticks_per_epoch: u64 = DEFAULT_SLOTS_PER_EPOCH * DEFAULT_TICKS_PER_SLOT;

    execute!(stdout(), SetForegroundColor(Color::Cyan), MoveTo(0, 9),).unwrap();
    println!("  |\nFinished:");
    println!("  | Received {} records", records_received);
    println!("  | {} slots", records_received / DEFAULT_TICKS_PER_SLOT);
    println!("  | Elapsed time: {:.2} seconds", seconds);
    println!("  | Average speed: {:.2} ticks/s", ticks_per_second);
    println!("  | For reference: 1 epoch = {} slots = {} ticks", DEFAULT_SLOTS_PER_EPOCH, ticks_per_epoch);
    execute!(stdout(), ResetColor).unwrap();
}
