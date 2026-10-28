#![forbid(unsafe_code)]

mod args;
mod bench;
mod commands;
mod file_watch;
mod playback;
mod preview;
mod source;
mod timeline;
mod watch;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match commands::run(&cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is::<bench::OverBudget>() => {
            eprintln!("error: {error}");
            ExitCode::from(4)
        }
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
