//! CLI argument parsing via clap v4 derive.
//!
//! The trickiest part of the surface is `--list [CATEGORY]`: clap needs to
//! distinguish "no flag", "flag without value", and "flag with value". We
//! express this as `Option<String>` with `num_args = 0..=1` and
//! `default_missing_value = "all"`, so:
//!
//! - `uconv`                       → `list == None`
//! - `uconv --list`                → `list == Some("all")`
//! - `uconv --list currency`       → `list == Some("currency")`
//!
//! `value` is kept as `Option<String>` (not `Option<f64>`) because the
//! positional may also be a natural-language string like `"100 km to miles"`.
//! We attempt float parsing later, where we can attach a better error.

use std::path::PathBuf;

use clap::Parser;

/// A fast, offline-capable unit converter.
///
/// Converts between units of currency, temperature, length, weight, volume,
/// and time. Currency rates are fetched live (cached for one hour); every
/// other category is purely offline.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "uconv",
    version,
    about,
    long_about = "uconv — unit converter for currency, temperature, distance, weight, volume, and time.\n\n\
                  Examples:\n  \
                  uconv 100 km mi          # 100 kilometres → miles\n  \
                  uconv 25 C F             # 25 °C → °F\n  \
                  uconv 50 USD EUR         # 50 USD → EUR (live rate)\n  \
                  uconv \"100 km to miles\" # natural language\n  \
                  uconv --repl             # interactive mode\n  \
                  uconv --list currency    # list units in a category\n  \
                  uconv --file batch.txt   # batch mode",
    // Allow negative numbers as positional args (e.g. `uconv -40 C F`).
    // Without this, clap treats `-40` as an unknown flag.
    allow_negative_numbers = true,
)]
pub struct Args {
    /// Value to convert (or a natural-language phrase in quotes).
    ///
    /// When `from` and `to` are omitted, `value` is parsed as natural
    /// language, e.g. `"100 km to miles"`.
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,

    /// Source unit (e.g. `km`, `C`, `USD`).
    #[arg(value_name = "FROM")]
    pub from: Option<String>,

    /// Target unit (e.g. `mi`, `F`, `EUR`).
    #[arg(value_name = "TO")]
    pub to: Option<String>,

    /// Start an interactive REPL.
    #[arg(short, long)]
    pub repl: bool,

    /// List supported units. With no value, lists every category;
    /// with a category name, lists only that category.
    #[arg(short, long, num_args = 0..=1, default_missing_value = "all")]
    pub list: Option<String>,

    /// Use cached currency rates only; never touch the network.
    #[arg(long)]
    pub offline: bool,

    /// Force a fresh currency-rate fetch, ignoring cache freshness.
    #[arg(long)]
    pub refresh: bool,

    /// Number of decimal places in the output (default: auto-trim).
    #[arg(long, value_name = "N")]
    pub precision: Option<usize>,

    /// Print the result in scientific notation (e.g. `1.234e2`).
    #[arg(long)]
    pub scientific: bool,

    /// Read one conversion per line from the given file and run them all.
    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Verbose output: show the conversion chain and rate timestamp.
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress the rate timestamp on currency conversions.
    #[arg(long, conflicts_with = "verbose")]
    pub quiet: bool,
}

impl Args {
    /// True if any mode flag (`--repl`, `--list`, `--file`) was used, meaning
    /// the positional `<value> <from> <to>` triple should be ignored.
    pub fn is_mode_flag_set(&self) -> bool {
        self.repl || self.list.is_some() || self.file.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn direct_three_positional() {
        let a = Args::try_parse_from(["uconv", "100", "km", "mi"]).unwrap();
        assert_eq!(a.value.as_deref(), Some("100"));
        assert_eq!(a.from.as_deref(), Some("km"));
        assert_eq!(a.to.as_deref(), Some("mi"));
    }

    #[test]
    fn natural_language_in_value() {
        let a = Args::try_parse_from(["uconv", "100 km to miles"]).unwrap();
        assert_eq!(a.value.as_deref(), Some("100 km to miles"));
        assert!(a.from.is_none());
        assert!(a.to.is_none());
    }

    #[test]
    fn list_no_category() {
        let a = Args::try_parse_from(["uconv", "--list"]).unwrap();
        assert_eq!(a.list.as_deref(), Some("all"));
    }

    #[test]
    fn list_with_category() {
        let a = Args::try_parse_from(["uconv", "--list", "currency"]).unwrap();
        assert_eq!(a.list.as_deref(), Some("currency"));
    }

    #[test]
    fn precision_and_scientific_flags() {
        let a = Args::try_parse_from([
            "uconv",
            "1.5",
            "kg",
            "lb",
            "--precision",
            "3",
            "--scientific",
        ])
        .unwrap();
        assert_eq!(a.precision, Some(3));
        assert!(a.scientific);
    }

    #[test]
    fn quiet_conflicts_with_verbose() {
        // clap should reject --verbose --quiet as a conflict.
        let r = Args::try_parse_from(["uconv", "--verbose", "--quiet", "1", "m", "ft"]);
        assert!(r.is_err());
    }
}
