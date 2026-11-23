//! Integration tests that exercise uconv's public library API.
//!
//! Each test isolates the cache directory to a tempdir via the per-thread
//! `cache::set_cache_dir` override, so the user's real cache is never touched
//! and parallel tests don't race on env vars. Network is replaced with
//! wiremock where needed.

use clap::Parser;
use uconv::cli::Args;
use uconv::{api, cache, dispatch, units};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Set the per-thread cache directory to a fresh tempdir.
///
/// We use `cache::set_cache_dir` (thread-local) instead of `XDG_CACHE_HOME`
/// (process-global) so parallel tests don't race on env vars.
fn isolate_cache() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    cache::set_cache_dir(Some(tmp.path().to_path_buf()));
    tmp
}

fn args() -> Args {
    Args::try_parse_from(["uconv"]).unwrap()
}

#[tokio::test]
async fn direct_length_conversion() {
    let _cache = isolate_cache();
    let out = dispatch::convert_value(100.0, "km", "mi", &args())
        .await
        .unwrap();
    assert!(out.contains("100"));
    assert!(out.contains("km"));
    assert!(out.contains("mi"));
    assert!(out.contains("62.13")); // 100 km = 62.1371... mi
}

#[tokio::test]
async fn direct_temperature_conversion() {
    let _cache = isolate_cache();
    let out = dispatch::convert_value(25.0, "C", "F", &args())
        .await
        .unwrap();
    assert!(out.contains("77")); // 25 °C = 77 °F
}

#[tokio::test]
async fn natural_language_via_convert_line() {
    let _cache = isolate_cache();
    let out = dispatch::convert_line("100 km to miles", &args())
        .await
        .unwrap();
    assert!(out.contains("62.13"));
}

#[tokio::test]
async fn rejects_unknown_unit_with_suggestion() {
    let _cache = isolate_cache();
    // "metr" is a typo of "meter"; we expect a suggestion back.
    let err = dispatch::convert_value(100.0, "metr", "mi", &args())
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("metr"));
    assert!(msg.contains("meter"));
}

#[tokio::test]
async fn rejects_category_mismatch() {
    let _cache = isolate_cache();
    let err = dispatch::convert_value(100.0, "km", "kg", &args())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("cannot convert"));
}

#[tokio::test]
async fn rejects_below_absolute_zero() {
    let _cache = isolate_cache();
    let err = dispatch::convert_value(-500.0, "C", "F", &args())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("absolute zero"));
}

#[tokio::test]
async fn precision_flag_controls_decimals() {
    let _cache = isolate_cache();
    let a = Args::try_parse_from(["uconv", "--precision", "3"]).unwrap();
    let out = dispatch::convert_value(1.0, "kg", "lb", &a).await.unwrap();
    assert!(out.contains("2.205")); // 1 kg = 2.205 lb to 3dp
}

#[tokio::test]
async fn scientific_flag_uses_e_notation() {
    let _cache = isolate_cache();
    let a = Args::try_parse_from(["uconv", "--scientific"]).unwrap();
    let out = dispatch::convert_value(1000.0, "m", "mm", &a)
        .await
        .unwrap();
    assert!(out.contains('e'));
}

#[tokio::test]
async fn currency_uses_mocked_api_and_shows_timestamp() {
    let _cache = isolate_cache();
    let server = MockServer::start().await;
    let body = r#"{
        "result": "success",
        "base_code": "USD",
        "rates": { "USD": 1.0, "EUR": 0.92, "GBP": 0.79 }
    }"#;
    Mock::given(method("GET"))
        .and(path("/v6/latest/USD"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    // Fetch via the testable URL-injection API and populate the cache.
    let rates = api::fetch_rates_with(&format!("{}/v6/latest/USD", server.uri()))
        .await
        .unwrap();
    cache::save(&rates).unwrap();

    // Default mode: should not hit network because cache is fresh.
    let out = dispatch::convert_value(100.0, "USD", "EUR", &args())
        .await
        .unwrap();
    assert!(out.contains("92"));
    assert!(out.contains("EUR"));
    assert!(out.contains("rates as of"));
}

#[tokio::test]
async fn currency_offline_with_no_cache_errors() {
    let _cache = isolate_cache();
    // Ensure no cache file exists.
    let path = cache::cache_path().unwrap();
    let _ = std::fs::remove_file(&path);

    let a = Args::try_parse_from(["uconv", "--offline"]).unwrap();
    let err = dispatch::convert_value(100.0, "USD", "EUR", &a)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("rates unavailable"));
}

#[tokio::test]
async fn currency_offline_uses_stale_cache() {
    let _cache = isolate_cache();

    // Pre-populate cache with stale rates (2 hours old).
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    let mut rates_map = HashMap::new();
    rates_map.insert("USD".to_string(), 1.0);
    rates_map.insert("EUR".to_string(), 0.92);
    let stale = uconv::convert::Rates {
        base: "USD".into(),
        timestamp: Utc::now() - Duration::hours(2),
        rates: rates_map,
    };
    cache::save(&stale).unwrap();

    let a = Args::try_parse_from(["uconv", "--offline"]).unwrap();
    let out = dispatch::convert_value(100.0, "USD", "EUR", &a)
        .await
        .unwrap();
    assert!(out.contains("92"));
}

#[tokio::test]
async fn batch_mode_processes_lines() {
    let _cache = isolate_cache();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("batch.txt");
    std::fs::write(&path, "# comment\n100 km mi\n25 C F\nbad line here\n").unwrap();
    let a = args();
    let code = dispatch::batch(&path, &a).await.unwrap();
    // One bad line → exit code 1.
    assert_eq!(code, 1);
}

#[tokio::test]
async fn batch_mode_all_valid_exits_zero() {
    let _cache = isolate_cache();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("batch.txt");
    std::fs::write(&path, "100 km mi\n25 C F\n1 kg lb\n").unwrap();
    let code = dispatch::batch(&path, &args()).await.unwrap();
    assert_eq!(code, 0);
}

#[tokio::test]
async fn list_does_not_panic() {
    units::list("all");
    units::list("currency");
    units::list("temperature");
    units::list("nonexistent"); // should still print all categories
}
