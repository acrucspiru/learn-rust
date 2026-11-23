//! uconv — a fast, offline-capable unit converter.
//!
//! The library exposes the conversion modules, the cache layer, and the CLI
//! dispatch logic so that integration tests can exercise them directly. The
//! binary (`src/main.rs`) is a thin wrapper that parses args and calls
//! [`run`].

pub mod api;
pub mod cache;
pub mod cli;
pub mod convert;
pub mod dispatch;
pub mod error;
pub mod format;
pub mod repl;
pub mod units;

use clap::Parser;
use owo_colors::OwoColorize;
use std::process::ExitCode;

/// Library entry point used by `main.rs`. Parses the args and dispatches
/// based on the mode flags.
pub async fn run_main() -> ExitCode {
    let args = cli::Args::parse();
    match run(args).await {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            // Colorize the "error:" prefix when stderr is a TTY. owo-colors
            // respects the `NO_COLOR` env var by default on newer versions.
            if std::env::var("NO_COLOR").is_ok() {
                eprintln!("uconv: error: {e}");
            } else {
                eprintln!("uconv: {}: {e}", "error".red().bold());
            }
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Dispatch the parsed args to the right mode.
pub async fn run(args: cli::Args) -> Result<i32, error::Error> {
    if args.repl {
        return repl::run(args).await;
    }

    if let Some(category) = &args.list {
        units::list(category);
        return Ok(error::exit::SUCCESS);
    }

    if let Some(path) = args.file.clone() {
        return dispatch::batch(&path, &args).await;
    }

    dispatch::direct(&args).await
}
