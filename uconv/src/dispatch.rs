//! Conversion dispatch.
//!
//! This module bridges the CLI and the conversion modules: it parses the
//! positional arguments, decides which category a conversion belongs to, and
//! produces the final user-facing output string.
//!
//! Three entry points:
//!
//! - [`direct`] — the `uconv <value> <from> <to>` form.
//! - [`natural`] — the `uconv "100 km to miles"` form.
//! - [`batch`] — `uconv --file path.txt`.

use crate::cli::Args;
use crate::error::{exit, Error, Result};
use crate::format::format_value;
use crate::units::{self, Convert, Unit};

/// Exit-code-bearing result of a single conversion.
pub type ConversionOutcome = std::result::Result<i32, Error>;

/// Run a single direct conversion from the CLI positional args.
pub async fn direct(args: &Args) -> ConversionOutcome {
    let (value_str, from, to) = match (&args.value, &args.from, &args.to) {
        (Some(v), Some(f), Some(t)) => (v.clone(), f.clone(), t.clone()),
        (Some(v), None, None) => return natural(v, args).await,
        _ => {
            return Err(Error::InvalidInput(
                "usage: uconv <value> <from> <to>\n  e.g. uconv 100 km mi".into(),
            ))
        }
    };

    let value: f64 = value_str
        .parse()
        .map_err(|_| Error::InvalidInput(format!("'{value_str}' is not a valid number")))?;

    let output = convert_value(value, &from, &to, args).await?;
    println!("{output}");
    Ok(exit::SUCCESS)
}

/// Parse a natural-language string like `"100 km to miles"` and convert.
///
/// Accepts `to`, `in`, and `->` as separators. The number can be anywhere in
/// the left half; we look for the last token that parses as `f64` and treat
/// the following token as the source unit.
pub async fn natural(input: &str, args: &Args) -> ConversionOutcome {
    let lower = input.to_lowercase();
    let sep = if lower.contains(" to ") {
        " to "
    } else if lower.contains(" in ") {
        " in "
    } else if lower.contains(" -> ") {
        " -> "
    } else if lower.contains(" as ") {
        " as "
    } else {
        return Err(Error::InvalidInput(format!(
            "could not parse '{input}'. Expected `<value> <from> to <to>`"
        )));
    };

    // Find the original (non-lowercased) split point.
    let sep_pos = lower.find(sep).expect("checked above");
    let left = &input[..sep_pos];
    let right = &input[sep_pos + sep.len()..];

    // Left half: "<number> <unit>" — find the rightmost numeric token.
    let tokens: Vec<&str> = left.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err(Error::InvalidInput(format!(
            "could not parse '{input}': expected `<value> <from>` before the separator"
        )));
    }
    let value_token = tokens[tokens.len() - 2];
    let from_token = tokens[tokens.len() - 1];
    let value: f64 = value_token
        .parse()
        .map_err(|_| Error::InvalidInput(format!("'{value_token}' is not a valid number")))?;

    let to_token = right
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::InvalidInput(format!("missing target unit in '{input}'")))?;

    let output = convert_value(value, from_token, to_token, args).await?;
    println!("{output}");
    Ok(exit::SUCCESS)
}

/// Run every conversion in a batch file, one per line.
pub async fn batch(path: &std::path::Path, args: &Args) -> ConversionOutcome {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("could not read batch file {}: {e}", path.display()),
        ))
    })?;
    let mut failures = 0;
    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Each line is either `value from to` or `value from to <to>`.
        match parse_line(trimmed) {
            LineParse::Triplet { value, from, to } => {
                match convert_value(value, &from, &to, args).await {
                    Ok(out) => println!("{out}"),
                    Err(e) => {
                        eprintln!("line {}: error: {e}", lineno + 1);
                        failures += 1;
                    }
                }
            }
            LineParse::Natural(s) => match natural(&s, args).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("line {}: error: {e}", lineno + 1);
                    failures += 1;
                }
            },
            LineParse::Error(e) => {
                eprintln!("line {}: {e}", lineno + 1);
                failures += 1;
            }
        }
    }
    if failures > 0 {
        Ok(exit::GENERIC)
    } else {
        Ok(exit::SUCCESS)
    }
}

enum LineParse {
    Triplet {
        value: f64,
        from: String,
        to: String,
    },
    Natural(String),
    Error(Error),
}

fn parse_line(s: &str) -> LineParse {
    if s.contains(' ')
        && (s.to_lowercase().contains(" to ")
            || s.to_lowercase().contains(" in ")
            || s.to_lowercase().contains(" -> "))
    {
        return LineParse::Natural(s.to_string());
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 3 {
        match parts[0].parse::<f64>() {
            Ok(v) => {
                return LineParse::Triplet {
                    value: v,
                    from: parts[1].to_string(),
                    to: parts[2].to_string(),
                }
            }
            Err(_) => {
                return LineParse::Error(Error::InvalidInput(format!(
                    "'{}' is not a valid number",
                    parts[0]
                )))
            }
        }
    }
    LineParse::Error(Error::InvalidInput(format!(
        "expected `<value> <from> <to>` or `<value> <from> to <to>`, got '{s}'"
    )))
}

/// Parse a single line (REPL or batch) into a conversion result string.
///
/// This is the entry point the REPL uses: it does NOT print to stdout, it
/// just returns the formatted line (or an error).
pub async fn convert_line(line: &str, args: &Args) -> Result<String> {
    match parse_line(line) {
        LineParse::Triplet { value, from, to } => convert_value(value, &from, &to, args).await,
        LineParse::Natural(s) => {
            // Re-run the natural parser to extract (value, from, to) and call
            // convert_value. We don't go through natural() because that prints.
            let (value, from, to) = parse_natural(&s)?;
            convert_value(value, &from, &to, args).await
        }
        LineParse::Error(e) => Err(e),
    }
}

/// Extract (value, from, to) from a natural-language string.
fn parse_natural(input: &str) -> Result<(f64, String, String)> {
    let lower = input.to_lowercase();
    let sep = if lower.contains(" to ") {
        " to "
    } else if lower.contains(" in ") {
        " in "
    } else if lower.contains(" -> ") {
        " -> "
    } else if lower.contains(" as ") {
        " as "
    } else {
        return Err(Error::InvalidInput(format!(
            "could not parse '{input}'. Expected `<value> <from> to <to>`"
        )));
    };
    let sep_pos = lower.find(sep).expect("checked above");
    let left = &input[..sep_pos];
    let right = &input[sep_pos + sep.len()..];
    let tokens: Vec<&str> = left.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err(Error::InvalidInput(format!(
            "could not parse '{input}': expected `<value> <from>` before the separator"
        )));
    }
    let value_token = tokens[tokens.len() - 2];
    let from_token = tokens[tokens.len() - 1];
    let value: f64 = value_token
        .parse()
        .map_err(|_| Error::InvalidInput(format!("'{value_token}' is not a valid number")))?;
    let to_token = right
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::InvalidInput(format!("missing target unit in '{input}'")))?;
    Ok((value, from_token.to_string(), to_token.to_string()))
}

/// Core: turn a (value, from, to) triple into a printable result string.
///
/// Visible to other modules (`repl`) so they can reuse the same logic without
/// going through stdout.
pub async fn convert_value(value: f64, from: &str, to: &str, args: &Args) -> Result<String> {
    let from_unit = units::parse_unit(from)?;
    let to_unit = units::parse_unit(to)?;

    let from_cat = from_unit.category();
    let to_cat = to_unit.category();
    if from_cat != to_cat {
        return Err(Error::CategoryMismatch {
            from: from_cat,
            to: to_cat,
        });
    }

    match (from_unit, to_unit) {
        (Unit::Temperature(f), Unit::Temperature(t)) => {
            let result = crate::convert::temperature::convert(value, f, t)?;
            Ok(format_line(
                value,
                f.symbol(),
                result,
                t.symbol(),
                args,
                None,
            ))
        }
        (Unit::Length(f), Unit::Length(t)) => {
            let result = t.from_base(f.to_base(value));
            Ok(format_line(
                value,
                f.symbol(),
                result,
                t.symbol(),
                args,
                None,
            ))
        }
        (Unit::Weight(f), Unit::Weight(t)) => {
            let result = t.from_base(f.to_base(value));
            Ok(format_line(
                value,
                f.symbol(),
                result,
                t.symbol(),
                args,
                None,
            ))
        }
        (Unit::Volume(f), Unit::Volume(t)) => {
            let result = t.from_base(f.to_base(value));
            Ok(format_line(
                value,
                f.symbol(),
                result,
                t.symbol(),
                args,
                None,
            ))
        }
        (Unit::Time(f), Unit::Time(t)) => {
            let result = t.from_base(f.to_base(value));
            Ok(format_line(
                value,
                f.symbol(),
                result,
                t.symbol(),
                args,
                None,
            ))
        }
        (Unit::Currency(from_code), Unit::Currency(to_code)) => {
            let rates = crate::cache::get_rates(args.offline, args.refresh).await?;
            let result = rates.convert(value, &from_code, &to_code)?;
            let ts = if args.quiet {
                None
            } else {
                Some(rates.timestamp)
            };
            Ok(format_line(value, &from_code, result, &to_code, args, ts))
        }
        // Unreachable: we already returned CategoryMismatch above.
        _ => unreachable!("category mismatch should have been caught earlier"),
    }
}

/// Format a single conversion result line.
fn format_line(
    value: f64,
    from_sym: &str,
    result: f64,
    to_sym: &str,
    args: &Args,
    rate_ts: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    let val_str = format_value(value, args);
    let res_str = format_value(result, args);
    let main = format!("{val_str} {from_sym} = {res_str} {to_sym}");

    if args.verbose {
        let mut s = main;
        if let Some(ts) = rate_ts {
            s.push_str(&format!(
                "  [rates as of {}]",
                ts.format("%Y-%m-%d %H:%M UTC")
            ));
        }
        s
    } else if let Some(ts) = rate_ts {
        format!("{main}  (rates as of {})", ts.format("%Y-%m-%d %H:%M UTC"))
    } else {
        main
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args() -> Args {
        Args::try_parse_from(["uconv"]).unwrap()
    }

    #[tokio::test]
    async fn converts_km_to_mi() {
        let s = convert_value(100.0, "km", "mi", &args()).await.unwrap();
        assert!(s.contains("100"));
        assert!(s.contains("km"));
        assert!(s.contains("mi"));
    }

    #[tokio::test]
    async fn rejects_category_mismatch() {
        let err = convert_value(100.0, "km", "kg", &args()).await.unwrap_err();
        assert!(matches!(err, Error::CategoryMismatch { .. }));
    }

    #[tokio::test]
    async fn rejects_unknown_unit() {
        // "zz" is not 3 letters so it won't be mistaken for a currency code.
        let err = convert_value(100.0, "zz", "mi", &args()).await.unwrap_err();
        assert!(matches!(
            err,
            Error::UnknownUnit { .. } | Error::InvalidInput(_)
        ));
    }

    #[tokio::test]
    async fn natural_language_basic() {
        let tmp = tempfile::tempdir().unwrap();
        crate::cache::set_cache_dir(Some(tmp.path().to_path_buf()));

        // Avoid going to the network for non-currency conversions.
        let r = natural("100 km to miles", &args()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn natural_language_in_separator() {
        let tmp = tempfile::tempdir().unwrap();
        crate::cache::set_cache_dir(Some(tmp.path().to_path_buf()));

        let r = natural("1 m in ft", &args()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn rejects_bad_number() {
        let err = natural("banana km to mi", &args()).await.unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }
}
