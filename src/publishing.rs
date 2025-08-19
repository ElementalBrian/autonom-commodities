// src/publishing.rs
use crate::types::{IndexTick, FundingUpdate};

#[async_trait::async_trait]
pub trait Publisher: Send + Sync + 'static {
    /// Publish an index/mark tick (you can sign & persist inside)
    async fn publish_index(&self, tick: IndexTick) -> anyhow::Result<()>;
    /// Publish funding update snapshots (e.g., every 8h)
    async fn publish_funding(&self, fu: FundingUpdate) -> anyhow::Result<()>;
}

/// Example in-memory stub. Replace with your Web2 cache/signature path.
pub struct StdoutPublisher;

#[async_trait::async_trait]
impl Publisher for StdoutPublisher {
    async fn publish_index(&self, tick: IndexTick) -> anyhow::Result<()> {
        println!("[INDEX] {} {}e{} @{} src={} twap={}s",
            tick.symbol, tick.price, tick.expo, tick.ts_ms, tick.source, tick.window_sec);
        Ok(())
    }
    async fn publish_funding(&self, fu: FundingUpdate) -> anyhow::Result<()> {
        println!("[FUNDING] {} rate={} interval={}s @{}",
            fu.symbol, fu.rate, fu.interval_sec, fu.ts_ms);
        Ok(())
    }
}

