//! Conversion modules.
//!
//! Each submodule owns one physical category. [`currency::Rates`] is the only
//! type here that crosses the boundary into the cache layer, because currency
//! rates are dynamic and need to be persisted.

pub mod currency;
pub mod length;
pub mod temperature;
pub mod time;
pub mod volume;
pub mod weight;

pub use currency::Rates;
