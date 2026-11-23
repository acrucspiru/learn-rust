//! On-disk cache for currency rates.
//!
//! # Layout
//!
//! The cache lives at `<cache-dir>/uconv/rates.json`, where `<cache-dir>` is
//! the platform-appropriate location (e.g. `~/.cache` on Linux, resolved via
//! the `directories` crate so we don't have to guess).
//!
//! # Freshness
//!
//! A cache entry is "fresh" if it's younger than [`CACHE_TTL`] (1 hour). In
//! `--offline` mode we use the cache regardless of freshness; in online mode
//! we re-fetch when stale.
//!
//! # Atomic writes
//!
//! We never overwrite the existing cache file in place — instead we write to
//! `rates.json.tmp` and rename. This means a crash mid-write cannot leave a
//! truncated cache behind.

use crate::api;
use crate::convert::Rates;
use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use std::cell::RefCell;
use std::path::PathBuf;

/// How long a cached rate set is considered fresh.
pub const CACHE_TTL: Duration = Duration::hours(1);

// Tests set this thread-local to override the cache directory, so parallel
// tests don't race on `XDG_CACHE_HOME` (which is process-global).
thread_local! {
    static CACHE_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Override the cache directory for the current thread.
///
/// Production code never calls this. Tests use it to isolate each test's
/// cache to a fresh tempdir, avoiding races on `XDG_CACHE_HOME`.
pub fn set_cache_dir(dir: Option<PathBuf>) {
    CACHE_OVERRIDE.with(|c| *c.borrow_mut() = dir);
}

/// Resolve the cache directory, creating it if necessary.
///
/// Uses the thread-local override if set (tests), otherwise the platform
/// default from the `directories` crate.
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = CACHE_OVERRIDE.with(|c| c.borrow().clone()) {
        std::fs::create_dir_all(&dir)?;
        return Ok(dir);
    }
    use directories::ProjectDirs;
    let proj = ProjectDirs::from("", "", "uconv")
        .ok_or_else(|| Error::Cache("cannot determine cache directory (HOME missing?)".into()))?;
    let dir = proj.cache_dir().to_path_buf();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Full path to the cache file.
pub fn cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("rates.json"))
}

/// Load rates from the cache. Returns `Ok(None)` if there is no cache file;
/// returns `Err` for read or parse failures (and removes a corrupt file).
pub fn load() -> Result<Option<Rates>> {
    let path = cache_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&path)?;
    match serde_json::from_str::<Rates>(&contents) {
        Ok(rates) => Ok(Some(rates)),
        Err(e) => {
            // Corrupt cache: warn via error, then remove so the next run is clean.
            let _ = std::fs::remove_file(&path);
            Err(Error::Cache(format!(
                "cache file was corrupt and has been removed: {e}"
            )))
        }
    }
}

/// Persist `rates` to the cache via atomic rename.
pub fn save(rates: &Rates) -> Result<()> {
    let path = cache_path()?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(rates)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Is the cached snapshot still within its TTL?
pub fn is_fresh(rates: &Rates) -> bool {
    let age = Utc::now().signed_duration_since(rates.timestamp);
    age < CACHE_TTL
}

/// Resolve rates, honouring `--offline` and `--refresh`.
///
/// Decision table:
///
/// | offline | refresh | action                                              |
/// |---------|---------|-----------------------------------------------------|
/// | false   | false   | use cache if fresh, else fetch                       |
/// | false   | true    | always fetch (and update cache)                     |
/// | true    | false   | use cache (fresh or stale); error if absent         |
/// | true    | true    | error: cannot refresh while offline                 |
pub async fn get_rates(offline: bool, refresh: bool) -> Result<Rates> {
    if offline && refresh {
        return Err(Error::Cache(
            "cannot refresh rates in --offline mode".into(),
        ));
    }

    if !refresh {
        if let Some(cached) = load()? {
            if is_fresh(&cached) || offline {
                return Ok(cached);
            }
        }
    }

    if offline {
        // We get here only if the cache was empty AND we're offline.
        return Err(Error::RatesUnavailable(
            "no cached rates available; run `uconv --refresh` while online to populate".into(),
        ));
    }

    let rates = api::fetch_rates().await?;
    save(&rates)?;
    Ok(rates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn mock_rates() -> Rates {
        let mut rates = HashMap::new();
        rates.insert("USD".to_string(), 1.0);
        rates.insert("EUR".to_string(), 0.92);
        Rates {
            base: "USD".into(),
            timestamp: Utc::now(),
            rates,
        }
    }

    fn mock_rates_aged(age: chrono::Duration) -> Rates {
        let mut r = mock_rates();
        r.timestamp = Utc::now() - age;
        r
    }

    #[test]
    fn freshness_within_ttl() {
        let r = mock_rates_aged(Duration::minutes(30));
        assert!(is_fresh(&r));
    }

    #[test]
    fn freshness_expired() {
        let r = mock_rates_aged(Duration::hours(2));
        assert!(!is_fresh(&r));
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        set_cache_dir(Some(tmp.path().to_path_buf()));

        let r = mock_rates();
        save(&r).expect("save failed");
        let loaded = load().expect("load failed").expect("cache was empty");
        assert_eq!(loaded.base, "USD");
        assert!(loaded.rates.contains_key("EUR"));
    }

    #[test]
    fn load_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        set_cache_dir(Some(tmp.path().to_path_buf()));
        assert!(load().unwrap().is_none());
    }

    #[test]
    fn corrupt_cache_is_removed() {
        let tmp = tempfile::tempdir().unwrap();
        set_cache_dir(Some(tmp.path().to_path_buf()));
        std::fs::write(tmp.path().join("rates.json"), "this is not json").unwrap();
        let res = load();
        assert!(res.is_err());
        assert!(!tmp.path().join("rates.json").exists());
    }

    #[tokio::test]
    async fn offline_refresh_is_error() {
        let err = get_rates(true, true).await.unwrap_err();
        assert!(matches!(err, Error::Cache(_)));
    }
}
