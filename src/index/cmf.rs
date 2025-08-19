// src/index/cmf.rs
use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CmfInputs};

pub struct CmfIndex {
    pub symbol: String,
    pub expo: i8,
    pub target_days: f64, // 30d
}

impl CmfIndex {
    fn weight(target_days: f64, f1_expiry: i64, f2_expiry: i64, now_ms: i64) -> f64 {
        let day_ms = 86_400_000f64;
        let t1 = ((f1_expiry - now_ms) as f64 / day_ms).max(1.0);
        let t2 = ((f2_expiry - now_ms) as f64 / day_ms).max(t1 + 1.0);
        let w = ((t2 - target_days) / (t2 - t1)).clamp(0.0, 1.0);
        w
    }
}

impl IndexBuilder<CmfInputs> for CmfIndex {
    fn build(&mut self, inp: CmfInputs) -> Result<IndexTick, IndexError> {
        let now = chrono::Utc::now().timestamp_millis();
        let w = Self::weight(self.target_days, inp.f1.expiry_ts_ms, inp.f2.expiry_ts_ms, now);
        let px = w * inp.f1.price + (1.0 - w) * inp.f2.price;
        Ok(IndexTick {
            symbol: self.symbol.clone(),
            price: px,
            expo: self.expo,
            ts_ms: now,
            source: "cmf",
            window_sec: 0,
        })
    }
}

