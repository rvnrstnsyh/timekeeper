pub enum OutputType {
    Terminal,
    JsonFile(String),
}

pub fn print_usage() {
    println!("Proof of History (PoH) with output options");
    println!("Usage:");
    println!("  Run this program [--json=FILENAME]");
    println!();
    println!("Options:");
    println!("--json=FILENAME     Save output in JSON format to FILENAME");
    println!("Without options     Print output to terminal (default)");
}

pub fn parse_args(args: &[String]) -> Result<OutputType, String> {
    let mut output_type: OutputType = OutputType::Terminal;
    // Check arguments for output mode.
    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Err(String::from("Help requested"));
        } else if arg.starts_with("--json=") {
            let filename: String = arg.replace("--json=", "");
            if filename.is_empty() {
                return Err(String::from("Error: The filename cannot be empty"));
            }
            output_type = OutputType::JsonFile(filename);
        } else {
            return Err(format!("Unrecognized argument: {}", arg));
        }
    }
    return Ok(output_type);
}
