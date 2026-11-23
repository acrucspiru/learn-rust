//! Error types and exit codes for uconv.
//!
//! The error model is intentionally narrow: one enum, `Error`, that maps
//! cleanly to exit codes via [`Error::exit_code`]. We use `thiserror` so the
//! `Display` impl is derived declaratively — no hand-written match arms that
//! can drift from the variants.

use crate::units::Category;

/// Every fallible operation in uconv returns `Result<T, Error>`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A network failure (DNS, connection refused, timeout, bad HTTP status).
    #[error("network error: {0}")]
    Network(String),

    /// The cache file is missing, unreadable, or corrupt.
    #[error("cache error: {0}")]
    Cache(String),

    /// The user gave us a value or unit string we cannot interpret.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The unit name was not recognised. We carry a suggested replacement
    /// computed via Levenshtein distance so the message is actionable.
    #[error("unknown unit '{unit}'. Did you mean '{suggestion}'?")]
    UnknownUnit { unit: String, suggestion: String },

    /// The user passed a temperature below absolute zero for that scale.
    #[error("value {value} is below absolute zero on the {unit} scale")]
    BelowAbsoluteZero { value: f64, unit: String },

    /// The user tried to convert between units of two different categories
    /// (e.g. `100 kg mi`).
    #[error("cannot convert between {from} and {to}")]
    CategoryMismatch { from: Category, to: Category },

    /// No cached currency rates are available and we're offline.
    #[error("currency rates unavailable: {0}")]
    RatesUnavailable(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Exit codes mandated by the CLI spec.
///
/// Kept as a module of `pub const` values rather than an enum so callers can
/// `return exit::NETWORK` directly without an `as u8` cast site.
pub mod exit {
    pub const SUCCESS: i32 = 0;
    pub const GENERIC: i32 = 1;
    pub const NETWORK: i32 = 2;
    pub const ARGS: i32 = 3;
}

impl Error {
    /// Map an error to the process exit code the spec requires.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Network(_) | Self::Http(_) | Self::RatesUnavailable(_) => exit::NETWORK,
            Self::InvalidInput(_)
            | Self::UnknownUnit { .. }
            | Self::BelowAbsoluteZero { .. }
            | Self::CategoryMismatch { .. } => exit::ARGS,
            Self::Cache(_) | Self::Io(_) | Self::Json(_) => exit::GENERIC,
        }
    }
}

// ---------------------------------------------------------------------
// Fuzzy matching for unit suggestions
// ---------------------------------------------------------------------

/// Classic Wagner-Fischer Levenshtein edit distance.
///
/// Used only for short strings (unit names) so an O(m*n) DP table is fine.
/// We deliberately do not pull in a crate for ~15 lines of code.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Previous row, reused across iterations to keep memory at O(n).
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Pick the closest candidate to `input` by edit distance, capped at 3.
///
/// Returning `None` when nothing is close prevents us from suggesting
/// "kilogram" when the user typed "zzz" — that would be actively unhelpful.
pub fn suggest<'a, I>(input: &str, candidates: I) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let lower = input.to_lowercase();
    candidates
        .into_iter()
        .map(|c| (c, levenshtein(&lower, &c.to_lowercase())))
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basics() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        // "metre" vs "meter": two substitutions (positions 3 and 4 swap).
        assert_eq!(levenshtein("metre", "meter"), 2);
    }

    #[test]
    fn suggest_picks_closest() {
        let cands = ["meter", "mile", "minute", "month"];
        assert_eq!(suggest("metre", cands).as_deref(), Some("meter"));
        // "mi" is closest to "mile" (distance 2).
        assert_eq!(suggest("mi", cands).as_deref(), Some("mile"));
        assert_eq!(suggest("xyzzy", cands), None);
    }

    #[test]
    fn exit_codes_match_spec() {
        assert_eq!(Error::Network("x".into()).exit_code(), exit::NETWORK);
        assert_eq!(Error::InvalidInput("x".into()).exit_code(), exit::ARGS);
        assert_eq!(Error::Cache("x".into()).exit_code(), exit::GENERIC);
    }
}
