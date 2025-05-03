use std::env;
use std::io::stdout;
use std::sync::mpsc::Receiver;

use timekeeper::constants::BATCH_SIZE;
use timekeeper::helpers::args::{OutputType, parse_args, print_usage};
use timekeeper::helpers::io::save_poh_records_to_json;
use timekeeper::poh::core::PoHRecord;
use timekeeper::poh::thread::thread;

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};

/// Runs a Proof of History (PoH) generator with event insertion and prints the PoH records to stdout.
/// The PoH generator is started with a given seed and maximum number of ticks, and records are received
/// over a channel and printed to stdout until the generator finishes. The number of records received is
/// printed after the generator finishes.
fn main() -> () {
    // Process command line arguments.
    let args: Vec<String> = env::args().collect();
    let output_type: OutputType = match parse_args(&args) {
        Ok(out) => out,
        Err(msg) => {
            println!("{}", msg);
            print_usage();
            return;
        }
    };

    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();

    let seed: &[u8; 64] = &[b'0'; 64];
    let max_ticks: u64 = 48_000; // Generate 48000 ticks (750 slots, 5 minutes).

    let rx: Receiver<PoHRecord> = thread(seed, max_ticks);
    let mut records_received: i32 = 0;
    let mut all_records: Vec<PoHRecord> = Vec::new();

    match output_type {
        OutputType::Terminal => {
            // Terminal mode - print directly to console.
            while let Ok(record) = rx.recv() {
                // Show progress per batch.
                if records_received as usize % BATCH_SIZE == 0 {
                    println!("{}", record);
                }
                records_received += 1;
            }
        }
        // cargo run -- --json=output.json
        OutputType::JsonFile(filename) => {
            // JSON mode - collect all records first.
            while let Ok(record) = rx.recv() {
                all_records.push(record.clone());
                // Show progress per batch.
                if records_received as usize % BATCH_SIZE == 0 {
                    println!("{}", record);
                }
                records_received += 1;
            }
            match save_poh_records_to_json(&all_records, &filename) {
                Ok(_) => println!("Successfully saved {} records to file {}.", records_received, filename),
                Err(e) => println!("Error saving file: {}", e),
            }
        }
    }
    return println!("PoH generator finished, received {} records.", records_received);
}
