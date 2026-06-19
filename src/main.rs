mod app;
mod elf;
mod ui;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: elf-insight <elf-file>");
        process::exit(1);
    }
    let file_path = &args[1];

    let data = elf::parser::parse_elf(file_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse ELF file: {}", e);
            process::exit(1);
        });

    app::run_app(data).unwrap_or_else(|e| {
        eprintln!("Application error: {}", e);
        process::exit(1);
    });
}