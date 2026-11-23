//! Temperature units: Celsius, Fahrenheit, Kelvin, Rankine.
//!
//! # Why temperature is "special"
//!
//! Most categories convert via a single multiplicative factor (`to_base(v) =
//! v * factor`). Temperature needs an *affine* map: `K = C + 273.15`. The
//! `Convert` trait still works because `to_base`/`from_base` are arbitrary
//! functions, but we have to validate the result is physically meaningful
//! (Kelvin cannot be negative).

use crate::error::{Error, Result};
use crate::units::Convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temperature {
    Celsius,
    Fahrenheit,
    Kelvin,
    Rankine,
}

impl Convert for Temperature {
    /// Base unit is **Kelvin**.
    fn to_base(&self, v: f64) -> f64 {
        match self {
            Self::Celsius => v + 273.15,
            Self::Fahrenheit => (v + 459.67) * 5.0 / 9.0,
            Self::Kelvin => v,
            Self::Rankine => v * 5.0 / 9.0,
        }
    }

    fn from_base(&self, v: f64) -> f64 {
        match self {
            Self::Celsius => v - 273.15,
            Self::Fahrenheit => v * 9.0 / 5.0 - 459.67,
            Self::Kelvin => v,
            Self::Rankine => v * 9.0 / 5.0,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Celsius => &["c", "celsius", "centigrade", "cent"],
            Self::Fahrenheit => &["f", "fahrenheit", "farenheit", "farenheight"],
            Self::Kelvin => &["k", "kelvin"],
            Self::Rankine => &["r", "ra", "rankine"],
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kelvin => "K",
            Self::Rankine => "°R",
        }
    }

    fn all() -> &'static [Self] {
        &[Self::Celsius, Self::Fahrenheit, Self::Kelvin, Self::Rankine]
    }
}

/// Convert `value` from `from` to `to`, rejecting sub-absolute-zero inputs.
///
/// We check *after* converting to Kelvin because each scale has a different
/// absolute zero, and Kelvin is the natural reference point.
pub fn convert(value: f64, from: Temperature, to: Temperature) -> Result<f64> {
    let k = from.to_base(value);
    if k < 0.0 {
        return Err(Error::BelowAbsoluteZero {
            value,
            unit: from.symbol().into(),
        });
    }
    Ok(to.from_base(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn c_to_f() {
        assert!(approx(
            convert(0.0, Temperature::Celsius, Temperature::Fahrenheit).unwrap(),
            32.0
        ));
        assert!(approx(
            convert(100.0, Temperature::Celsius, Temperature::Fahrenheit).unwrap(),
            212.0
        ));
    }

    #[test]
    fn f_to_c() {
        assert!(approx(
            convert(32.0, Temperature::Fahrenheit, Temperature::Celsius).unwrap(),
            0.0
        ));
        assert!(approx(
            convert(212.0, Temperature::Fahrenheit, Temperature::Celsius).unwrap(),
            100.0
        ));
    }

    #[test]
    fn c_to_k_round_trip() {
        for &c in &[-100.0, 0.0, 25.0, 100.0, 1000.0] {
            let k = convert(c, Temperature::Celsius, Temperature::Kelvin).unwrap();
            let back = convert(k, Temperature::Kelvin, Temperature::Celsius).unwrap();
            assert!(approx(back, c), "round trip failed for {c}");
        }
    }

    #[test]
    fn kelvin_to_rankine() {
        // 0 K = 0 R, 300 K ≈ 540 R
        assert!(approx(
            convert(0.0, Temperature::Kelvin, Temperature::Rankine).unwrap(),
            0.0
        ));
        assert!(approx(
            convert(300.0, Temperature::Kelvin, Temperature::Rankine).unwrap(),
            540.0
        ));
    }

    #[test]
    fn rejects_below_absolute_zero_celsius() {
        let err = convert(-300.0, Temperature::Celsius, Temperature::Fahrenheit).unwrap_err();
        assert!(matches!(err, Error::BelowAbsoluteZero { .. }));
    }

    #[test]
    fn rejects_below_absolute_zero_fahrenheit() {
        // -500 °F is below absolute zero.
        let err = convert(-500.0, Temperature::Fahrenheit, Temperature::Celsius).unwrap_err();
        assert!(matches!(err, Error::BelowAbsoluteZero { .. }));
    }

    #[test]
    fn allows_exactly_absolute_zero() {
        // 0 K is the floor, not an error.
        let r = convert(0.0, Temperature::Kelvin, Temperature::Celsius).unwrap();
        assert!(approx(r, -273.15));
    }

    #[test]
    fn alias_parsing() {
        use crate::units::parse;
        assert_eq!(parse::<Temperature>("c"), Some(Temperature::Celsius));
        assert_eq!(parse::<Temperature>("F"), Some(Temperature::Fahrenheit));
        assert_eq!(
            parse::<Temperature>("farenheit"),
            Some(Temperature::Fahrenheit)
        );
        assert_eq!(parse::<Temperature>("K"), Some(Temperature::Kelvin));
        assert_eq!(parse::<Temperature>("ra"), Some(Temperature::Rankine));
    }
}
