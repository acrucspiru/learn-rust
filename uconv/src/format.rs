//! Output formatting.
//!
//! The spec requires three modes:
//!
//! 1. **Auto** (default): trim trailing zeros, keep 6 decimals max, fall back
//!    to scientific notation for very large / very small magnitudes.
//! 2. **`--precision N`**: exactly N decimal places.
//! 3. **`--scientific`**: always use scientific notation.
//!
//! We deliberately use `format!` instead of `rust_decimal` or `bigdecimal` —
//! `f64` is more than enough precision for everyday conversion, and pulling
//! in arbitrary-precision crates would balloon the binary size.

use crate::cli::Args;

/// Format a numeric value according to the CLI options.
pub fn format_value(value: f64, args: &Args) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    if args.scientific {
        return format_scientific(value, args.precision);
    }
    if let Some(p) = args.precision {
        return format!("{:.*}", p, value);
    }
    auto_format(value)
}

/// Auto-trim: 6 decimals max, trailing zeros stripped, scientific for extremes.
fn auto_format(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if abs < 1e-4 || abs >= 1e15 {
        return format!("{:e}", value);
    }
    let s = format!("{:.6}", value);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Scientific notation. With `--precision`, controls the mantissa digits.
fn format_scientific(value: f64, precision: Option<usize>) -> String {
    match precision {
        Some(p) => format!("{:.*e}", p, value),
        None => format!("{:e}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args() -> Args {
        Args::try_parse_from(["uconv"]).unwrap()
    }

    fn args_prec(p: usize) -> Args {
        Args::try_parse_from(["uconv", "--precision", &p.to_string()]).unwrap()
    }

    fn args_sci() -> Args {
        Args::try_parse_from(["uconv", "--scientific"]).unwrap()
    }

    #[test]
    fn auto_trims_trailing_zeros() {
        assert_eq!(format_value(1.50, &args()), "1.5");
        assert_eq!(format_value(100.0, &args()), "100");
        assert_eq!(format_value(0.0, &args()), "0");
    }

    #[test]
    fn auto_uses_scientific_for_extremes() {
        let s = format_value(1e20, &args());
        assert!(s.contains('e'));
        let s = format_value(1e-10, &args());
        assert!(s.contains('e'));
    }

    #[test]
    fn precision_mode_fixed_decimals() {
        assert_eq!(format_value(1.5, &args_prec(3)), "1.500");
        assert_eq!(format_value(1.0 / 3.0, &args_prec(4)), "0.3333");
    }

    #[test]
    fn scientific_mode() {
        let s = format_value(1234.5, &args_sci());
        assert!(s.contains('e'));
    }

    #[test]
    fn scientific_with_precision() {
        let s = format_value(
            1234.5,
            &Args::try_parse_from(["uconv", "--scientific", "--precision", "2"]).unwrap(),
        );
        // Should look like 1.23e3
        assert!(s.contains("e"));
    }

    #[test]
    fn nan_and_inf_handled() {
        assert_eq!(format_value(f64::NAN, &args()), "NaN");
        assert_eq!(format_value(f64::INFINITY, &args()), "inf");
        assert_eq!(format_value(f64::NEG_INFINITY, &args()), "-inf");
    }
}
