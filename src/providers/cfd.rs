use crate::providers::CfdProvider;
use crate::types::{CfdQuote, CfdSource};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rand::{rng, Rng};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Simple shared state for a random-walk price for the mock provider.
static OWNINJA_PX: OnceLock<Mutex<f64>> = OnceLock::new();

/// Live CFD provider backed by API Ninjas' /v1/commodityprice endpoint.
pub struct NinjasCfd {
    client: Client,
    api_key: String,
    /// Map your internal symbols to API Ninjas `name` values.
    sym_map: HashMap<String, &'static str>,
    base_url: String,
}

impl NinjasCfd {
    /// Reads API key from env. Supports `API_NINJAS_API_KEY` (preferred) and `API_NINJAS_KEY`.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("API_NINJAS_API_KEY")
            .or_else(|_| std::env::var("API_NINJAS_KEY"))
            .map_err(|_| anyhow!("Set API_NINJAS_API_KEY (or API_NINJAS_KEY)"))?;

        let mut sym_map = HashMap::new();
        // Extend as needed
        sym_map.insert("LEAN_HOGS_PERP".into(), "lean_hogs");
        sym_map.insert("LIVE_CATTLE_PERP".into(), "live_cattle");
        sym_map.insert("FEEDER_CATTLE_PERP".into(), "feeder_cattle");
        sym_map.insert("CORN_PERP".into(), "corn");
        sym_map.insert("SOYBEAN_PERP".into(), "soybean");
        sym_map.insert("WHEAT_PERP".into(), "wheat");
        sym_map.insert("COFFEE_PERP".into(), "coffee");
        sym_map.insert("COCOA_PERP".into(), "cocoa");
        sym_map.insert("SUGAR_PERP".into(), "sugar");
        sym_map.insert("GOLD_PERP".into(), "gold");
        sym_map.insert("SILVER_PERP".into(), "silver");

        let base_url = std::env::var("API_NINJAS_BASE_URL")
            .unwrap_or_else(|_| "https://api.api-ninjas.com".to_string());

        Ok(Self {
            client: Client::builder().user_agent("autonom-oracle/1.0").build()?,
            api_key,
            sym_map,
            base_url,
        })
    }

    fn map_symbol<'a>(&'a self, symbol: &str) -> Result<&'a str> {
        self.sym_map
            .get(symbol)
            .copied()
            .ok_or_else(|| anyhow!("unsupported symbol for API Ninjas: {}", symbol))
    }
}

#[derive(Debug, Deserialize)]
struct NinjasResp {
    // These fields are present but not strictly required downstream.
    exchange: Option<String>,
    name: String,
    price: f64,
    updated: i64, // unix seconds
}

#[async_trait::async_trait]
impl CfdProvider for NinjasCfd {
    fn name(&self) -> &'static str { "ninjas" }

    async fn latest(&self, symbol: &str) -> Result<CfdQuote> {
        let ninjas_name = self.map_symbol(symbol)?;
        let mut last_err: Option<anyhow::Error> = None;

        for (i, backoff_ms) in [0_u64, 250, 500, 1000].into_iter().enumerate() {
            if backoff_ms > 0 {
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            // Build request (use mock base_url in tests)
            let mut req = self
                .client
                .get(format!("{}/v1/commodityprice?name={}", self.base_url, ninjas_name))
                .header("X-Api-Key", &self.api_key);

            // Tag attempts in tests so httpmock can match deterministically
            #[cfg(test)]
            {
                req = req.header("X-Test-Attempt", i.to_string());
            }

            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => { last_err = Some(anyhow!(e)); continue; } // network error → retry
            };

            let status = resp.status();
            if status.is_success() {
                let data: NinjasResp = resp.json().await.context("decoding Ninjas JSON")?;
                if !(data.price.is_finite() && data.price > 0.0) {
                    return Err(anyhow!("API Ninjas returned invalid price: {}", data.price));
                }
                let ts_ms = if data.updated > 0 { data.updated * 1000 } else { Utc::now().timestamp_millis() };
                return Ok(CfdQuote { src: CfdSource::Ninjas, price: data.price, ts_ms });
            }

            // Retry on 429 and 5xx
            if status.as_u16() == 429 || status.is_server_error() {
                last_err = Some(anyhow!("HTTP {}", status));
                continue;
            }

            // Other 4xx are fatal (bad request, unauthorized, etc.)
            return Err(anyhow!("API Ninjas HTTP error: {}", status));
        }

        Err(anyhow!("ninjas request failed after retries: {:?}", last_err))
    }
}


/// Deterministic mock that produces a tiny random walk (useful as a fallback/consensus peer).
pub struct OwninjaCfd;

#[async_trait::async_trait]
impl CfdProvider for OwninjaCfd {
    fn name(&self) -> &'static str {
        "owninja"
    }

    async fn latest(&self, _symbol: &str) -> Result<CfdQuote> {
        let now = Utc::now().timestamp_millis();
        let px = {
            let m = OWNINJA_PX.get_or_init(|| Mutex::new(0.907));
            let mut p = m.lock().unwrap();
            let mut r = rng();
            // Small drift + bounded noise, using rand 0.9 API to avoid deprecation warnings.
            let shock: f64 = r.random_range(-0.0006..0.0006) + 0.00002;
            *p = (*p * (1.0 + shock)).max(0.1);
            *p
        };
        Ok(CfdQuote {
            src: CfdSource::Owninja,
            price: px,
            ts_ms: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{MockServer, Method::GET};

    // Helper to build a NinjasCfd pointing to our mock server
    fn client_pointing_to(server: &MockServer) -> NinjasCfd {
        std::env::set_var("API_NINJAS_API_KEY", "test_key");
        std::env::set_var("API_NINJAS_BASE_URL", server.base_url());
        let mut c = NinjasCfd::from_env().unwrap();
        c
    }

    #[tokio::test]
    async fn ninjas_happy_path() {
        let server = MockServer::start_async().await;

        // Return a realistic payload
        let m = server.mock_async(|when, then| {
            when.method(GET)
                .path("/v1/commodityprice")
                .query_param("name", "lean_hogs")
                .header("X-Api-Key", "test_key");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"exchange":"CME","name":"Lean Hogs Futures","price":89.5,"updated":1700000000}"#);
        }).await;

        let mut ninjas = client_pointing_to(&server);
        // Ensure symbol map contains LEAN_HOGS_PERP in your real code
        let q = ninjas.latest("LEAN_HOGS_PERP").await.unwrap();
        assert_eq!(q.price, 89.5);
        assert_eq!(q.ts_ms, 1700000000 * 1000);
        m.assert();
    }

    #[tokio::test]
    async fn ninjas_retries_then_succeeds() {
        let server = MockServer::start_async().await;

        // Attempt 0 -> 429
        let m0 = server.mock_async(|when, then| {
            when.method(GET)
                .path("/v1/commodityprice")
                .query_param("name", "lean_hogs")
                .header("X-Api-Key", "test_key")
                .header("X-Test-Attempt", "0");
            then.status(429);
        }).await;

        // Attempt 1 -> 429
        let m1 = server.mock_async(|when, then| {
            when.method(GET)
                .path("/v1/commodityprice")
                .query_param("name", "lean_hogs")
                .header("X-Api-Key", "test_key")
                .header("X-Test-Attempt", "1");
            then.status(429);
        }).await;

        // Attempt 2 -> 200
        let mok = server.mock_async(|when, then| {
            when.method(GET)
                .path("/v1/commodityprice")
                .query_param("name", "lean_hogs")
                .header("X-Api-Key", "test_key")
                .header("X-Test-Attempt", "2");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"exchange":null,"name":"Lean Hogs Futures","price":90.0,"updated":1700001234}"#);
        }).await;

        let ninjas = client_pointing_to(&server);
        let q = ninjas.latest("LEAN_HOGS_PERP").await.unwrap();
        assert_eq!(q.price, 90.0);
        assert_eq!(q.ts_ms, 1700001234 * 1000);

        // sanity: verify hits
        m0.assert_hits(1);
        m1.assert_hits(1);
        mok.assert_hits(1);
    }

}
