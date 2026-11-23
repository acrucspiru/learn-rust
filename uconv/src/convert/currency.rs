//! Currency conversion.
//!
//! # Model
//!
//! Rates are stored relative to a single base currency (typically USD). The
//! API returns `rates[code]` meaning "1 base = rates[code] code". Conversion
//! between any two currencies is then:
//!
//! ```text
//!   amount_in_to = amount_in_from * (rates[to] / rates[from])
//! ```
//!
//! This works whether `from`/`to` is the base or not, because the base's own
//! rate is always 1.0.
//!
//! `Rates` is a plain serialisable struct so the cache module can stash it as
//! JSON verbatim.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A snapshot of currency rates from the API, ready to use or persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rates {
    /// The base currency the rates are expressed against (usually `"USD"`).
    pub base: String,
    /// When this snapshot was fetched. Used to decide cache freshness.
    pub timestamp: DateTime<Utc>,
    /// Map of currency code → units per 1 base currency.
    pub rates: HashMap<String, f64>,
}

impl Rates {
    /// Convert `value` from one currency to another using these rates.
    ///
    /// Both codes are upper-cased before lookup so the user can type `usd`,
    /// `Usd`, or `USD` interchangeably.
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Result<f64> {
        let from = from.to_uppercase();
        let to = to.to_uppercase();

        let from_rate = self.rates.get(&from).ok_or_else(|| {
            Error::InvalidInput(format!(
                "currency '{from}' is not in the current rate set. \
                 Run `uconv --refresh` to update."
            ))
        })?;
        let to_rate = self.rates.get(&to).ok_or_else(|| {
            Error::InvalidInput(format!(
                "currency '{to}' is not in the current rate set. \
                 Run `uconv --refresh` to update."
            ))
        })?;

        if *from_rate == 0.0 || *to_rate == 0.0 {
            return Err(Error::InvalidInput(format!(
                "rate for '{from}' or '{to}' is zero in cache; refresh recommended"
            )));
        }

        Ok(value * (to_rate / from_rate))
    }

    /// True if `code` is present in this rate set.
    pub fn has(&self, code: &str) -> bool {
        self.rates.contains_key(&code.to_uppercase())
    }

    /// A representative list of ISO-4217 codes — used for `--list currency`,
    /// for fuzzy-match suggestions, and as a sanity check on the API response.
    /// The actual set may be larger (whatever the API returns).
    pub const COMMON_CODES: &'static [&'static str] = &[
        "USD", "EUR", "GBP", "JPY", "AUD", "CAD", "CHF", "CNY", "HKD", "NZD", "SEK", "KRW", "SGD",
        "NOK", "MXN", "INR", "RUB", "BRL", "ZAR", "TRY", "AED", "THB", "IDR", "MYR", "PHP", "PLN",
        "DKK", "CZK", "HUF", "ILS", "CLP", "PEN", "COP", "ARS", "SAR", "EGP", "PKR", "BDT", "VND",
        "KZT",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    fn mock_rates() -> Rates {
        let mut rates = HashMap::new();
        rates.insert("USD".to_string(), 1.0);
        rates.insert("EUR".to_string(), 0.92);
        rates.insert("GBP".to_string(), 0.79);
        rates.insert("JPY".to_string(), 149.5);
        Rates {
            base: "USD".into(),
            timestamp: Utc::now(),
            rates,
        }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn usd_to_eur() {
        let r = mock_rates();
        assert!(approx(r.convert(100.0, "USD", "EUR").unwrap(), 92.0));
    }

    #[test]
    fn eur_to_usd() {
        let r = mock_rates();
        assert!(approx(r.convert(92.0, "EUR", "USD").unwrap(), 100.0));
    }

    #[test]
    fn cross_currency_eur_to_gbp() {
        // 100 EUR → USD (100/0.92) → GBP (* 0.79) = 85.8695...
        let r = mock_rates();
        let v = r.convert(100.0, "EUR", "GBP").unwrap();
        assert!(approx(v, 100.0 * (0.79 / 0.92)));
    }

    #[test]
    fn case_insensitive() {
        let r = mock_rates();
        assert!(approx(r.convert(100.0, "usd", "eur").unwrap(), 92.0));
        assert!(approx(r.convert(100.0, "Usd", "eUr").unwrap(), 92.0));
    }

    #[test]
    fn unknown_currency_errors() {
        let r = mock_rates();
        let err = r.convert(100.0, "USD", "XYZ").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn jpy_round_trip() {
        let r = mock_rates();
        let v = 1000.0;
        let jpy = r.convert(v, "USD", "JPY").unwrap();
        let back = r.convert(jpy, "JPY", "USD").unwrap();
        assert!(approx(back, v));
    }

    #[test]
    fn has_works_case_insensitively() {
        let r = mock_rates();
        assert!(r.has("usd"));
        assert!(r.has("EUR"));
        assert!(!r.has("xyz"));
    }

    #[test]
    fn serialises_and_deserialises() {
        let r = mock_rates();
        let json = serde_json::to_string(&r).unwrap();
        let back: Rates = serde_json::from_str(&json).unwrap();
        assert_eq!(back.base, "USD");
        assert!(back.has("EUR"));
    }
}
