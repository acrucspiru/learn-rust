//! Length units. Base unit: **metre**.
//!
//! Conversion factors are exact SI definitions where they exist (inch = 25.4 mm
//! exactly, foot = 0.3048 m exactly, etc.). The nautical mile is the
//! international definition (1852 m).

use crate::units::Convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    Mm,
    Cm,
    M,
    Km,
    Inch,
    Foot,
    Yard,
    Mile,
    NauticalMile,
}

impl Convert for Length {
    fn to_base(&self, v: f64) -> f64 {
        // base: metres
        match self {
            Self::Mm => v * 1e-3,
            Self::Cm => v * 1e-2,
            Self::M => v,
            Self::Km => v * 1e3,
            Self::Inch => v * 0.0254,
            Self::Foot => v * 0.3048,
            Self::Yard => v * 0.9144,
            Self::Mile => v * 1609.344,
            Self::NauticalMile => v * 1852.0,
        }
    }

    fn from_base(&self, v: f64) -> f64 {
        match self {
            Self::Mm => v * 1e3,
            Self::Cm => v * 1e2,
            Self::M => v,
            Self::Km => v * 1e-3,
            Self::Inch => v / 0.0254,
            Self::Foot => v / 0.3048,
            Self::Yard => v / 0.9144,
            Self::Mile => v / 1609.344,
            Self::NauticalMile => v / 1852.0,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Mm => &["mm", "millimeter", "millimetre", "millimeters"],
            Self::Cm => &["cm", "centimeter", "centimetre"],
            Self::M => &["m", "meter", "metre", "meters"],
            Self::Km => &["km", "kilometer", "kilometre"],
            Self::Inch => &["in", "inch", "inches", "\""],
            Self::Foot => &["ft", "foot", "feet", "'"],
            Self::Yard => &["yd", "yard", "yards"],
            Self::Mile => &["mi", "mile", "miles"],
            Self::NauticalMile => &["nmi", "nauticalmile", "nauticalmiles"],
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::M => "m",
            Self::Km => "km",
            Self::Inch => "in",
            Self::Foot => "ft",
            Self::Yard => "yd",
            Self::Mile => "mi",
            Self::NauticalMile => "nmi",
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::Mm,
            Self::Cm,
            Self::M,
            Self::Km,
            Self::Inch,
            Self::Foot,
            Self::Yard,
            Self::Mile,
            Self::NauticalMile,
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
    fn km_to_mi() {
        // 100 km = 62.137... mi
        let mi = Length::Km.from_base(Length::Km.to_base(100.0));
        let _ = mi; // noop round trip
        let result = Length::Mile.from_base(Length::Km.to_base(100.0));
        assert!(approx(result, 62.1371192237334));
    }

    #[test]
    fn mile_to_km_round_trip() {
        for &v in &[0.0, 1.0, 42.195, 100.0, 1000.0] {
            let m = Length::Mile.to_base(v);
            let back = Length::Mile.from_base(m);
            assert!(approx(back, v), "round trip failed for {v}");
        }
    }

    #[test]
    fn inch_to_cm() {
        // 1 inch = 2.54 cm exactly
        let cm = Length::Cm.from_base(Length::Inch.to_base(1.0));
        assert!(approx(cm, 2.54));
    }

    #[test]
    fn nautical_mile_to_m() {
        assert!(approx(Length::NauticalMile.to_base(1.0), 1852.0));
    }

    #[test]
    fn zero_and_large_values() {
        assert_eq!(Length::Km.to_base(0.0), 0.0);
        assert!(Length::Km.to_base(1e9).is_finite());
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(parse::<Length>("m"), Some(Length::M));
        assert_eq!(parse::<Length>("meter"), Some(Length::M));
        assert_eq!(parse::<Length>("metre"), Some(Length::M));
        assert_eq!(parse::<Length>("\""), Some(Length::Inch));
        assert_eq!(parse::<Length>("nmi"), Some(Length::NauticalMile));
        assert_eq!(parse::<Length>("xyz"), None);
    }
}
