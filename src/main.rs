mod app;
mod elf;
mod ui;

use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use std::env;
use std::io;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: elf-insight <elf-file>");
        process::exit(1);
    }
    let file_path = &args[1];

    let data = elf::parser::parse_elf(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to parse ELF file: {}", e);
        process::exit(1);
    });

    install_panic_hook();
    app::run_app(data).unwrap_or_else(|e| {
        eprintln!("Application error: {}", e);
        process::exit(1);
    });
}

/// Restores the terminal when a panic unwinds out of the event loop, which
/// skips the normal `disable_raw_mode` / `LeaveAlternateScreen` cleanup.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        default_hook(info);
    }));
}
