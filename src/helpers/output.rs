use std::fs::File;
use std::io::Write;

use crate::poh::core::PoHRecord;

/// Saves PoH records to a file in JSON format.
///
/// # Parameters
/// - `records`: The PoH records to be saved.
/// - `filename`: The name of the file to which the records should be saved.
///
/// # Returns
/// A Result indicating whether the save was successful.
pub fn save_records_to_json(records: &[PoHRecord], filename: &str) -> std::io::Result<()> {
    let json: String = serde_json::to_string_pretty(records)?;
    let mut file: File = File::create(format!("target/{}", filename))?;

    file.write_all(json.as_bytes())?;
    println!("Records saved to {}", filename);

    return Ok(());
}
