// src/funding.rs
use crate::types::{FundingUpdate, IndexTick};

pub struct FundingEngine {
    pub kappa: f64,
    pub cap: f64,
    pub interval_sec: u32,
}

impl FundingEngine {
    pub fn new(kappa: f64, cap: f64, interval_sec: u32) -> Self { Self { kappa, cap, interval_sec } }

    pub fn compute(&self, mark: &IndexTick, index_ref: &IndexTick) -> FundingUpdate {
        let basis = (mark.price - index_ref.price) / index_ref.price;
        let raw = self.kappa * basis;
        let rate = raw.clamp(-self.cap, self.cap);
        FundingUpdate {
            symbol: format!("{}-PERP", mark.symbol),
            rate,
            interval_sec: self.interval_sec,
            ts_ms: mark.ts_ms,
        }
    }
}

pub struct Ema {
    pub alpha: f64,       // 2/(N+1). For 60-min ref sampled per second, pick alpha≈2/3601
    pub value: Option<f64>,
}
impl Ema {
    pub fn new(alpha: f64) -> Self { Self { alpha, value: None } }
    pub fn update(&mut self, x: f64) -> f64 {
        self.value = Some(match self.value {
            None => x,
            Some(v) => self.alpha * x + (1.0 - self.alpha) * v,
        });
        self.value.unwrap()
    }
}

