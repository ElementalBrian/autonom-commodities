// src/providers/cme.rs
use super::{CmeProvider, ProviderError};
use crate::types::FuturesLeg;

pub struct DummyCme;

#[async_trait::async_trait]
impl CmeProvider for DummyCme {
    async fn latest_f1_f2(&self, _symbol: &str) -> Result<(FuturesLeg, FuturesLeg), ProviderError> {
        // TODO: wire to your CME market data (Polygon-like or direct)
        let now = chrono::Utc::now().timestamp_millis();
        Ok((
            FuturesLeg { price: 0.90, ts_ms: now, expiry_ts_ms: now + 20 * 86_400_000 },
            FuturesLeg { price: 0.92, ts_ms: now, expiry_ts_ms: now + 50 * 86_400_000 },
        ))
    }
}

