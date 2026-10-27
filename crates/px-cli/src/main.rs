//! `pxe`: renders KDL scenes to PNG stills and looping GIFs (`docs/CLI.md`).
//!
//! # Allocation
//!
//! Allocation-conscious application boundary.
#![forbid(unsafe_code)]

mod args;
mod commands;
mod source;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match commands::run(&cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
