//! The `Convert` trait, the `Category` enum, and the dispatcher that turns a
//! unit string into a typed unit.
//!
//! # Design
//!
//! Each physical category implements `Convert`. The trait exposes two numeric
//! methods (`to_base` / `from_base`) and three informational methods
//! (`aliases`, `symbol`, `all`). The base unit for each category is fixed:
//!
//! | Category    | Base unit |
//! |-------------|-----------|
//! | Length      | metre     |
//! | Weight      | gram      |
//! | Volume      | litre     |
//! | Time        | second    |
//! | Temperature | kelvin    |
//!
//! Currency does *not* implement `Convert` because its units are dynamic
//! (whatever the API returned) and its rates are not constant. It is handled
//! separately in [`crate::convert::currency`].

use crate::convert::{
    length::Length, temperature::Temperature, time::Time, volume::Volume, weight::Weight,
};
use crate::error::{suggest, Error};

/// Every category uconv can convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Temperature,
    Length,
    Weight,
    Volume,
    Time,
    Currency,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Length => "length",
            Self::Weight => "weight",
            Self::Volume => "volume",
            Self::Time => "time",
            Self::Currency => "currency",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Temperature,
            Self::Length,
            Self::Weight,
            Self::Volume,
            Self::Time,
            Self::Currency,
        ]
    }

    /// Parse a category name, accepting a few aliases (`temp`, `distance`,
    /// `mass`, `duration`, `money`).
    pub fn from_str_ci(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "temperature" | "temp" => Some(Self::Temperature),
            "length" | "distance" => Some(Self::Length),
            "weight" | "mass" => Some(Self::Weight),
            "volume" => Some(Self::Volume),
            "time" | "duration" => Some(Self::Time),
            "currency" | "money" => Some(Self::Currency),
            "all" => None, // caller treats None as "no filter"
            _ => None,
        }
    }
}

/// Convert between a unit and its canonical base unit.
///
/// Implemented by each category's enum. Note that this trait is *linear*
/// (`to_base(2 * v) == 2 * to_base(v)`); temperature violates that, so its
/// `to_base` is mathematically valid (C → K is affine, not linear) but we
/// still surface it through the same trait for dispatch simplicity.
//
// Clippy objects to `from_base(&self, ...)` because `from_*` methods are
// conventionally constructors. The spec mandates these names, so we suppress
// the lint at the trait level.
#[allow(clippy::wrong_self_convention)]
pub trait Convert: Sized + std::fmt::Debug + Copy + PartialEq + Eq {
    /// Convert `v` (expressed in *this* unit) to the base unit.
    fn to_base(&self, v: f64) -> f64;

    /// Convert `v` (expressed in the *base* unit) back to this unit.
    fn from_base(&self, v: f64) -> f64;

    /// Lowercase aliases the parser accepts for this unit. The first entry
    /// is treated as the canonical name for display in `--list`.
    fn aliases(&self) -> &'static [&'static str];

    /// Short symbol used in output (e.g. `"m"`, `"°C"`, `"kg"`).
    fn symbol(&self) -> &'static str;

    /// All variants of the implementing enum, in `--list` order.
    fn all() -> &'static [Self];
}

/// Case-insensitive lookup: try every alias of every variant of `C`.
pub fn parse<C: Convert + 'static>(s: &str) -> Option<C> {
    let lower = s.to_lowercase();
    C::all()
        .iter()
        .find(|u| u.aliases().iter().any(|a| *a == lower))
        .copied()
}

/// A typed unit, regardless of category. Returned by [`parse_unit`].
#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Temperature(Temperature),
    Length(Length),
    Weight(Weight),
    Volume(Volume),
    Time(Time),
    Currency(String),
}

impl Unit {
    pub fn category(&self) -> Category {
        match self {
            Self::Temperature(_) => Category::Temperature,
            Self::Length(_) => Category::Length,
            Self::Weight(_) => Category::Weight,
            Self::Volume(_) => Category::Volume,
            Self::Time(_) => Category::Time,
            Self::Currency(_) => Category::Currency,
        }
    }

    pub fn symbol(&self) -> String {
        match self {
            Self::Temperature(t) => t.symbol().into(),
            Self::Length(l) => l.symbol().into(),
            Self::Weight(w) => w.symbol().into(),
            Self::Volume(v) => v.symbol().into(),
            Self::Time(t) => t.symbol().into(),
            Self::Currency(c) => c.clone(),
        }
    }
}

/// Try to interpret `s` as a unit of any category.
///
/// Order matters: we try the physical categories first (because their aliases
/// are unambiguous), and fall back to currency only when `s` looks like a
/// 3-letter ISO-4217 code. This avoids `min` being mis-parsed as a currency.
pub fn parse_unit(s: &str) -> Result<Unit, Error> {
    if let Some(t) = parse::<Temperature>(s) {
        return Ok(Unit::Temperature(t));
    }
    if let Some(l) = parse::<Length>(s) {
        return Ok(Unit::Length(l));
    }
    if let Some(w) = parse::<Weight>(s) {
        return Ok(Unit::Weight(w));
    }
    if let Some(v) = parse::<Volume>(s) {
        return Ok(Unit::Volume(v));
    }
    if let Some(t) = parse::<Time>(s) {
        return Ok(Unit::Time(t));
    }
    // Currency: 3 ASCII letters, case-insensitive. We accept any such code
    // and let the `Rates` lookup reject the ones the API doesn't know about.
    let upper = s.to_uppercase();
    if upper.len() == 3 && upper.chars().all(|c| c.is_ascii_alphabetic()) {
        return Ok(Unit::Currency(upper));
    }

    Err(unknown_unit_error(s))
}

/// Build an "Unknown unit" error with a suggestion drawn from every alias we
/// know plus the common ISO-4217 codes.
pub fn unknown_unit_error(unit: &str) -> Error {
    let mut candidates: Vec<&str> = Vec::new();
    for u in Temperature::all() {
        candidates.extend_from_slice(u.aliases());
    }
    for u in Length::all() {
        candidates.extend_from_slice(u.aliases());
    }
    for u in Weight::all() {
        candidates.extend_from_slice(u.aliases());
    }
    for u in Volume::all() {
        candidates.extend_from_slice(u.aliases());
    }
    for u in Time::all() {
        candidates.extend_from_slice(u.aliases());
    }
    candidates.extend_from_slice(crate::convert::currency::Rates::COMMON_CODES);

    if let Some(sug) = suggest(unit, candidates.iter().copied()) {
        Error::UnknownUnit {
            unit: unit.to_string(),
            suggestion: sug,
        }
    } else {
        Error::InvalidInput(format!("Unknown unit '{unit}'"))
    }
}

/// Print every unit (optionally filtered by category) to stdout.
pub fn list(filter: &str) {
    let cat = Category::from_str_ci(filter);
    for &c in Category::all() {
        if let Some(f) = cat {
            if f != c {
                continue;
            }
        }
        println!("{}", c.as_str());
        match c {
            Category::Temperature => print_category(Temperature::all()),
            Category::Length => print_category(Length::all()),
            Category::Weight => print_category(Weight::all()),
            Category::Volume => print_category(Volume::all()),
            Category::Time => print_category(Time::all()),
            Category::Currency => {
                for code in crate::convert::currency::Rates::COMMON_CODES {
                    println!("  {code}");
                }
                println!("  (…plus any others returned by the API)");
            }
        }
        println!();
    }
}

fn print_category<C: Convert>(units: &[C]) {
    for u in units {
        let primary = u.aliases().first().copied().unwrap_or("");
        let rest = u
            .aliases()
            .iter()
            .skip(1)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        // NOTE: brackets would be cleaner, but `[m` is a valid ANSI SGR reset
        // sequence, so any unit whose first alias starts with 'm' would have
        // its alias list partially eaten by the terminal. Use parentheses.
        println!("  {:<14} {:<6}  ({})", primary, u.symbol(), rest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_length_aliases() {
        assert_eq!(parse::<Length>("km"), Some(Length::Km));
        assert_eq!(parse::<Length>("KM"), Some(Length::Km));
        assert_eq!(parse::<Length>("kilometre"), Some(Length::Km));
        assert_eq!(parse::<Length>("zzz"), None);
    }

    #[test]
    fn parse_unit_dispatch() {
        assert!(matches!(parse_unit("km"), Ok(Unit::Length(Length::Km))));
        assert!(matches!(parse_unit("C"), Ok(Unit::Temperature(_))));
        assert!(matches!(parse_unit("eur"), Ok(Unit::Currency(_))));
        assert!(matches!(parse_unit("min"), Ok(Unit::Time(Time::Minute))));
    }

    #[test]
    fn unknown_unit_suggests() {
        // "metr" is a typo of "meter" / "metre" (both valid aliases).
        match parse_unit("metr") {
            Err(Error::UnknownUnit { suggestion, .. }) => {
                assert_eq!(suggestion, "meter");
            }
            other => panic!("expected UnknownUnit, got {other:?}"),
        }
    }

    #[test]
    fn unknown_unit_no_suggestion_for_garbage() {
        // Levenshtein distance > 3 → falls back to plain InvalidInput.
        assert!(matches!(parse_unit("zzzzzzz"), Err(Error::InvalidInput(_))));
    }
}
