//! Weight/mass units. Base unit: **gram**.
//!
//! `Tonne` is the metric tonne (1000 kg), not the US short ton. The
//! avoirdupois constants are NIST-defined: 1 lb = 453.59237 g exactly,
//! 1 oz = 1/16 lb, 1 stone = 14 lb.

use crate::units::Convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Gram,
    Kilogram,
    Ounce,
    Pound,
    Stone,
    Tonne,
}

impl Convert for Weight {
    fn to_base(&self, v: f64) -> f64 {
        // base: grams
        match self {
            Self::Gram => v,
            Self::Kilogram => v * 1e3,
            Self::Ounce => v * 28.349523125,
            Self::Pound => v * 453.59237,
            Self::Stone => v * 6350.29318, // 14 * 453.59237
            Self::Tonne => v * 1e6,
        }
    }

    fn from_base(&self, v: f64) -> f64 {
        match self {
            Self::Gram => v,
            Self::Kilogram => v * 1e-3,
            Self::Ounce => v / 28.349523125,
            Self::Pound => v / 453.59237,
            Self::Stone => v / 6350.29318,
            Self::Tonne => v * 1e-6,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Gram => &["g", "gram", "grams", "gramme", "grammes"],
            Self::Kilogram => &["kg", "kilo", "kilogram", "kilogramme"],
            Self::Ounce => &["oz", "ounce", "ounces"],
            Self::Pound => &["lb", "lbs", "pound", "pounds"],
            Self::Stone => &["st", "stone", "stones"],
            Self::Tonne => &["t", "ton", "tonne", "metricton", "metrictons"],
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Self::Gram => "g",
            Self::Kilogram => "kg",
            Self::Ounce => "oz",
            Self::Pound => "lb",
            Self::Stone => "st",
            Self::Tonne => "t",
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::Gram,
            Self::Kilogram,
            Self::Ounce,
            Self::Pound,
            Self::Stone,
            Self::Tonne,
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
    fn kg_to_lb() {
        // 1 kg = 2.20462262 lb
        let lb = Weight::Pound.from_base(Weight::Kilogram.to_base(1.0));
        assert!(approx(lb, 2.2046226218487757));
    }

    #[test]
    fn round_trip_all_units() {
        for u in Weight::all() {
            for &v in &[0.0, 1.0, 100.0, 1e6] {
                let base = u.to_base(v);
                let back = u.from_base(base);
                assert!(approx(back, v), "round trip failed for {u:?} @ {v}");
            }
        }
    }

    #[test]
    fn stone_to_pound() {
        // 1 stone = 14 lb
        let lb = Weight::Pound.from_base(Weight::Stone.to_base(1.0));
        assert!(approx(lb, 14.0));
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(parse::<Weight>("kg"), Some(Weight::Kilogram));
        assert_eq!(parse::<Weight>("lbs"), Some(Weight::Pound));
        assert_eq!(parse::<Weight>("tonne"), Some(Weight::Tonne));
    }
}
