// src/providers/cfd.rs
use super::{CfdProvider, ProviderError};
use crate::types::CfdTick;

pub struct NinjasCfd {
    pub http: reqwest::Client,
    pub api_key: String,
}

#[async_trait::async_trait]
impl CfdProvider for NinjasCfd {
    async fn latest_tick(&self, _symbol: &str) -> Result<CfdTick, ProviderError> {
        // TODO: call your CFD endpoint, parse into CfdTick
        // Example shape (pseudo):
        // let url = format!("https://api.api-ninjas.com/v1/commodities?name=lean_hogs");
        // let json = self.http.get(url).header("X-Api-Key", &self.api_key).send().await?...;
        let now = chrono::Utc::now().timestamp_millis();
        Ok(CfdTick { price: 0.905, ts_ms: now })
    }
}

pub struct OwninjaCfd {
    pub http: reqwest::Client,
    pub api_key: Option<String>,
}

#[async_trait::async_trait]
impl CfdProvider for OwninjaCfd {
    async fn latest_tick(&self, _symbol: &str) -> Result<CfdTick, ProviderError> {
        let now = chrono::Utc::now().timestamp_millis();
        // TODO: call your endpoint; map units and scale
        Ok(CfdTick { price: 0.907, ts_ms: now })
    }
}
