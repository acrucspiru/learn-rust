//! Volume units. Base unit: **litre**.
//!
//! All US customary units use their NIST-defined exact equivalents in litres.
//! Imperial (UK) units are intentionally NOT mixed in — mixing the two would
//! surprise users who type "cup" expecting the US cup.

use crate::units::Convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Volume {
    Milliliter,
    Liter,
    Teaspoon,
    Tablespoon,
    Cup,
    FluidOunce,
    Pint,
    Quart,
    Gallon,
}

impl Convert for Volume {
    fn to_base(&self, v: f64) -> f64 {
        // base: litres
        match self {
            Self::Milliliter => v * 1e-3,
            Self::Liter => v,
            Self::Teaspoon => v * 4.92892159375e-3,
            Self::Tablespoon => v * 1.478676478125e-2,
            Self::Cup => v * 0.2365882365,
            Self::FluidOunce => v * 2.95735295625e-2,
            Self::Pint => v * 0.473176473,
            Self::Quart => v * 0.946352946,
            Self::Gallon => v * 3.785411784,
        }
    }

    fn from_base(&self, v: f64) -> f64 {
        match self {
            Self::Milliliter => v * 1e3,
            Self::Liter => v,
            Self::Teaspoon => v / 4.92892159375e-3,
            Self::Tablespoon => v / 1.478676478125e-2,
            Self::Cup => v / 0.2365882365,
            Self::FluidOunce => v / 2.95735295625e-2,
            Self::Pint => v / 0.473176473,
            Self::Quart => v / 0.946352946,
            Self::Gallon => v / 3.785411784,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Milliliter => &["ml", "milliliter", "millilitre"],
            Self::Liter => &["l", "liter", "litre", "liters", "litres"],
            Self::Teaspoon => &["tsp", "teaspoon", "teaspoons"],
            Self::Tablespoon => &["tbsp", "tablespoon", "tablespoons"],
            Self::Cup => &["cup", "cups"],
            Self::FluidOunce => &["floz", "fl_oz", "fluidounce", "fluidounces"],
            Self::Pint => &["pt", "pint", "pints"],
            Self::Quart => &["qt", "quart", "quarts"],
            Self::Gallon => &["gal", "gallon", "gallons"],
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Self::Milliliter => "mL",
            Self::Liter => "L",
            Self::Teaspoon => "tsp",
            Self::Tablespoon => "tbsp",
            Self::Cup => "cup",
            Self::FluidOunce => "fl oz",
            Self::Pint => "pt",
            Self::Quart => "qt",
            Self::Gallon => "gal",
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::Milliliter,
            Self::Liter,
            Self::Teaspoon,
            Self::Tablespoon,
            Self::Cup,
            Self::FluidOunce,
            Self::Pint,
            Self::Quart,
            Self::Gallon,
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
    fn gallon_to_l() {
        // 1 US gallon = 3.785411784 L exactly
        let l = Volume::Liter.from_base(Volume::Gallon.to_base(1.0));
        assert!(approx(l, 3.785411784));
    }

    #[test]
    fn cup_to_tbsp() {
        // 1 US customary cup = 16 US customary tablespoons (8 fl oz × 2 tbsp)
        let tbsp = Volume::Tablespoon.from_base(Volume::Cup.to_base(1.0));
        assert!(approx(tbsp, 16.0));
    }

    #[test]
    fn round_trip_all_units() {
        for u in Volume::all() {
            for &v in &[0.0, 1.0, 100.0, 1e3] {
                let base = u.to_base(v);
                let back = u.from_base(base);
                assert!(approx(back, v), "round trip failed for {u:?} @ {v}");
            }
        }
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(parse::<Volume>("l"), Some(Volume::Liter));
        assert_eq!(parse::<Volume>("L"), Some(Volume::Liter));
        assert_eq!(parse::<Volume>("litre"), Some(Volume::Liter));
        assert_eq!(parse::<Volume>("floz"), Some(Volume::FluidOunce));
        assert_eq!(parse::<Volume>("gal"), Some(Volume::Gallon));
    }
}
