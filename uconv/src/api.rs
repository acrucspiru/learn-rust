//! Live currency-rate fetcher.
//!
//! We hit `https://open.er-api.com/v6/latest/USD` — it's free, requires no
//! API key, and returns a JSON document with `result`, `base_code`, and
//! `rates`. The endpoint has no SLA, so we set a 15-second timeout and surface
//! HTTP and parse failures as [`Error::Network`].
//!
//! [`fetch_rates_with`] is the testable core: it takes a URL so integration
//! tests can point it at a wiremock server. [`fetch_rates`] is the production
//! wrapper that always points at the real API.

use crate::convert::Rates;
use crate::error::{Error, Result};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;

const API_URL: &str = "https://open.er-api.com/v6/latest/USD";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// The subset of the API response we actually care about.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    result: String,
    #[allow(dead_code)]
    #[serde(default)]
    base_code: String,
    rates: HashMap<String, f64>,
}

/// Fetch a fresh rate snapshot from the default public API.
pub async fn fetch_rates() -> Result<Rates> {
    fetch_rates_with(API_URL).await
}

/// Fetch from an arbitrary URL. Production callers use [`fetch_rates`]; tests
/// pass a wiremock server URL.
pub async fn fetch_rates_with(url: &str) -> Result<Rates> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("uconv/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(Error::Network(format!(
            "API returned HTTP {}",
            resp.status()
        )));
    }

    let body: ApiResponse = resp.json().await?;
    if body.result != "success" {
        return Err(Error::Network(format!(
            "API reported non-success result: {}",
            body.result
        )));
    }

    let base = body.base_code;
    let mut rates = body.rates;
    // Ensure the base currency itself is in the map at 1.0. The API includes
    // it, but we don't want to trust that — division-by-base would panic.
    rates.entry(base.clone()).or_insert(1.0);

    Ok(Rates {
        base,
        timestamp: Utc::now(),
        rates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetch_rates_with_success() {
        let server = MockServer::start().await;
        let body = r#"{
            "result": "success",
            "base_code": "USD",
            "rates": { "USD": 1.0, "EUR": 0.92, "GBP": 0.79, "JPY": 149.5 }
        }"#;
        Mock::given(method("GET"))
            .and(path("/v6/latest/USD"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let url = format!("{}/v6/latest/USD", server.uri());
        let rates = fetch_rates_with(&url).await.expect("fetch should succeed");
        assert_eq!(rates.base, "USD");
        assert_eq!(rates.rates.get("EUR"), Some(&0.92));
        assert!(rates.has("JPY"));
    }

    #[tokio::test]
    async fn fetch_rates_http_500() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v6/latest/USD"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let url = format!("{}/v6/latest/USD", server.uri());
        let err = fetch_rates_with(&url).await.unwrap_err();
        assert!(matches!(err, Error::Network(_)));
    }

    #[tokio::test]
    async fn fetch_rates_non_success_field() {
        let server = MockServer::start().await;
        let body = r#"{ "result": "error", "base_code": "USD", "rates": {} }"#;
        Mock::given(method("GET"))
            .and(path("/v6/latest/USD"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let url = format!("{}/v6/latest/USD", server.uri());
        let err = fetch_rates_with(&url).await.unwrap_err();
        assert!(matches!(err, Error::Network(_)));
    }

    #[tokio::test]
    async fn fetch_rates_ensures_base_in_map() {
        // Even if the API forgets to include USD in `rates`, we add it.
        let server = MockServer::start().await;
        let body = r#"{
            "result": "success",
            "base_code": "USD",
            "rates": { "EUR": 0.92 }
        }"#;
        Mock::given(method("GET"))
            .and(path("/v6/latest/USD"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let url = format!("{}/v6/latest/USD", server.uri());
        let rates = fetch_rates_with(&url).await.unwrap();
        assert_eq!(rates.rates.get("USD"), Some(&1.0));
    }
}
