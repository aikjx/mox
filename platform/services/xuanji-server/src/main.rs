//! Xuanji v2.0 AIS-grade fusion single-binary entry point.
//!
//! Parses argv via `clap` derive, runs the CLI handler using [`xuanji_server::cli_run`]
//! and prints the resulting JSON summary (or error) to stdout.

use clap::Parser;
use std::process::ExitCode;

use xuanji_server::{cli_run, Cli, CliState};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let state = CliState::new();
    match cli_run(&cli, &state) {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).expect("json format"));
            ExitCode::from(0)
        }
        Err(e) => {
            eprintln!("xuanji-server: error: {e}");
            ExitCode::from(1)
        }
    }
}
