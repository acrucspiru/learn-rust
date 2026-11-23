//! Time units. Base unit: **second**.
//!
//! Month and year are not constant — different months have different lengths,
//! and there are leap years. We use the Julian year (365.25 days) and the
//! Julian month (year / 12) as the conventional approximations. This is what
//! most calculators do; we mention it explicitly so users aren't surprised.

use crate::units::Convert;

/// Julian year in seconds (365.25 days).
const YEAR_SECS: f64 = 31_557_600.0;
/// Julian month in seconds (YEAR_SECS / 12).
const MONTH_SECS: f64 = 2_629_800.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Time {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl Convert for Time {
    fn to_base(&self, v: f64) -> f64 {
        // base: seconds
        match self {
            Self::Second => v,
            Self::Minute => v * 60.0,
            Self::Hour => v * 3600.0,
            Self::Day => v * 86_400.0,
            Self::Week => v * 604_800.0,
            Self::Month => v * MONTH_SECS,
            Self::Year => v * YEAR_SECS,
        }
    }

    fn from_base(&self, v: f64) -> f64 {
        match self {
            Self::Second => v,
            Self::Minute => v / 60.0,
            Self::Hour => v / 3600.0,
            Self::Day => v / 86_400.0,
            Self::Week => v / 604_800.0,
            Self::Month => v / MONTH_SECS,
            Self::Year => v / YEAR_SECS,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Second => &["s", "sec", "secs", "second", "seconds"],
            Self::Minute => &["min", "mins", "minute", "minutes"],
            Self::Hour => &["h", "hr", "hrs", "hour", "hours"],
            Self::Day => &["d", "day", "days"],
            Self::Week => &["wk", "week", "weeks"],
            Self::Month => &["mo", "mon", "month", "months"],
            Self::Year => &["yr", "year", "years"],
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Self::Second => "s",
            Self::Minute => "min",
            Self::Hour => "h",
            Self::Day => "d",
            Self::Week => "wk",
            Self::Month => "mo",
            Self::Year => "yr",
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::Second,
            Self::Minute,
            Self::Hour,
            Self::Day,
            Self::Week,
            Self::Month,
            Self::Year,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::parse;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn hour_to_minute() {
        let m = Time::Minute.from_base(Time::Hour.to_base(1.0));
        assert!(approx(m, 60.0));
    }

    #[test]
    fn day_to_second() {
        let s = Time::Second.from_base(Time::Day.to_base(1.0));
        assert!(approx(s, 86_400.0));
    }

    #[test]
    fn year_in_seconds() {
        assert!(approx(Time::Year.to_base(1.0), YEAR_SECS));
        assert!(approx(YEAR_SECS, 31_557_600.0));
    }

    #[test]
    fn round_trip_all_units() {
        for u in Time::all() {
            for &v in &[0.0, 1.0, 100.0, 1e6] {
                let base = u.to_base(v);
                let back = u.from_base(base);
                assert!(approx(back, v), "round trip failed for {u:?} @ {v}");
            }
        }
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(parse::<Time>("h"), Some(Time::Hour));
        assert_eq!(parse::<Time>("min"), Some(Time::Minute));
        assert_eq!(parse::<Time>("mo"), Some(Time::Month));
        assert_eq!(parse::<Time>("yr"), Some(Time::Year));
    }
}
