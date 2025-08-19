// src/providers/mod.rs
use async_trait::async_trait;

use crate::types::CfdQuote;

#[async_trait]
pub trait CfdProvider: Send + Sync {
    /// Return the latest CFD quote for the symbol.
    async fn latest(&self, symbol: &str) -> Result<CfdQuote, anyhow::Error>;
}

#[async_trait]
pub trait CmeProvider: Send + Sync {
    async fn latest_f1_f2(&self, symbol: &str) -> Result<CfdQuote, anyhow::Error>;
}

pub mod cfd;
