//! Interactive REPL.
//!
//! A minimal read-eval-print loop built on `rustyline`. The REPL supports the
//! same conversion syntaxes as the CLI (`100 km mi` or `100 km to miles`),
//! plus three meta-commands prefixed with `:`:
//!
//! - `:list [category]` — list units
//! - `:help`            — show help
//! - `:quit` (or `:q`)  — exit
//!
//! Each line is dispatched through the same [`crate::dispatch`] path the CLI
//! uses, so behaviour is identical.

use crate::cli::Args;
use crate::error::{exit, Error, Result};
use crate::units;
use rustyline::error::ReadlineError;

const HELP: &str = "\
uconv REPL commands:
  <value> <from> <to>      Convert (e.g. `100 km mi`)
  <value> <from> to <to>   Natural language (e.g. `100 km to miles`)
  :list [category]         List units (default: all)
  :help                    Show this help
  :quit (or Ctrl-D)        Exit
";

pub async fn run(args: Args) -> Result<i32> {
    println!("uconv REPL — type a conversion, or `:help`. Ctrl-D to exit.");
    let mut rl = rustyline::DefaultEditor::new()
        .map_err(|e| Error::Cache(format!("could not initialise REPL: {e}")))?;

    loop {
        let line = match rl.readline("uconv> ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: clear the line and continue.
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                return Err(Error::Cache(format!("readline error: {e}")));
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(trimmed);

        if trimmed == ":quit" || trimmed == ":q" {
            break;
        }
        if trimmed == ":help" || trimmed == ":h" {
            println!("{HELP}");
            continue;
        }
        if trimmed == ":list" {
            units::list("all");
            continue;
        }
        if let Some(cat) = trimmed.strip_prefix(":list ") {
            units::list(cat.trim());
            continue;
        }

        match crate::dispatch::convert_line(trimmed, &args).await {
            Ok(out) => println!("{out}"),
            Err(e) => eprintln!("error: {e}"),
        }
    }
    Ok(exit::SUCCESS)
}
