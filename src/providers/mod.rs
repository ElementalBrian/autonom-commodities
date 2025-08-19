use anchor_lang::prelude::thiserror;
// src/providers/mod.rs
use crate::types::{FuturesLeg, CfdTick};
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("timeout")]
    Timeout,
    #[error("remote busy")]
    RemoteBusy,
    #[error("bad status: {0}")]
    BadStatus(u16),
    #[error("decode")]
    Decode,
    #[error("stale")]
    Stale,
    #[error("transport: {0}")]
    Transport(String),
}

#[async_trait]
pub trait CmeProvider: Send + Sync {
    async fn latest_f1_f2(&self, symbol: &str) -> Result<(FuturesLeg, FuturesLeg), ProviderError>;
}

#[async_trait]
pub trait CfdProvider: Send + Sync {
    async fn latest_tick(&self, symbol: &str) -> Result<CfdTick, ProviderError>;
}

pub mod cme;
pub mod cfd;

