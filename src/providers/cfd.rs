// src/providers/cfd.rs
use async_trait::async_trait;

use crate::providers::CfdProvider;
use crate::types::{CfdQuote, CfdSource};

pub struct NinjasCfd;

#[async_trait]
impl CfdProvider for NinjasCfd {
    async fn latest(&self, _symbol: &str) -> Result<CfdQuote, anyhow::Error> {
        let now = chrono::Utc::now().timestamp_millis();
        // TODO: call your real endpoint
        Ok(CfdQuote { src: CfdSource::Ninjas, price: 0.905, ts_ms: now })
    }
}

pub struct OwninjaCfd;

#[async_trait]
impl CfdProvider for OwninjaCfd {
    async fn latest(&self, _symbol: &str) -> Result<CfdQuote, anyhow::Error> {
        let now = chrono::Utc::now().timestamp_millis();
        // TODO: call your real endpoint
        Ok(CfdQuote { src: CfdSource::Owninja, price: 0.907, ts_ms: now })
    }
}
