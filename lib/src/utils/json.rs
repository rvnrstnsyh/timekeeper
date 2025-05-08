use std::fs::File;
use std::io::{Result, Write};

use serde::Serialize;
use serde_json::to_string_pretty;

pub fn write<T: Serialize>(records: &[T], filename: &str) -> Result<()> {
    let json: String = to_string_pretty(records)?;
    let mut file: File = File::create(format!("target/{}", filename))?;

    file.write_all(json.as_bytes())?;
    println!("Records saved to {}", filename);

    return Ok(());
}
