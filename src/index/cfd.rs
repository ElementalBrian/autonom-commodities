// src/index/cfd.rs
use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CfdTick};
use std::collections::VecDeque;

pub struct CfdIndex {
    pub symbol: String,
    pub expo: i8,
    pub twap_sec: u32,
    pub median_sec: u32,
    pub max_staleness_ms: u64,
    pub jump_pct: f64,
    // state
    buf: VecDeque<CfdTick>,
    last_px: Option<f64>,
}

impl CfdIndex {
    pub fn new(symbol: String, expo: i8, twap_sec: u32, median_sec: u32, max_staleness_ms: u64, jump_pct: f64) -> Self {
        Self { symbol, expo, twap_sec, median_sec, max_staleness_ms, jump_pct, buf: VecDeque::new(), last_px: None }
    }

    fn prune(&mut self, now_ms: i64) {
        let window_ms = (self.median_sec.max(self.twap_sec)) as i64 * 1000;
        while let Some(front) = self.buf.front() {
            if now_ms - front.ts_ms > window_ms { self.buf.pop_front(); } else { break; }
        }
    }

    fn median(&self) -> Option<f64> {
        if self.buf.is_empty() { return None; }
        let mut v: Vec<f64> = self.buf.iter().map(|t| t.price).collect();
        v.sort_by(|a,b| a.partial_cmp(b).unwrap());
        Some(v[v.len()/2])
    }

    fn twap(&self, now_ms: i64) -> Option<f64> {
        if self.buf.is_empty() { return None; }
        let window_ms = self.twap_sec as i64 * 1000;
        let mut num = 0.0;
        let mut den = 0.0;
        let mut last_ts = now_ms;
        for t in self.buf.iter().rev() {
            let dt = (last_ts - t.ts_ms).max(1) as f64;
            if now_ms - t.ts_ms > window_ms { break; }
            num += t.price * dt;
            den += dt;
            last_ts = t.ts_ms;
        }
        if den > 0.0 { Some(num/den) } else { None }
    }
}

impl IndexBuilder<CfdTick> for CfdIndex {
    fn build(&mut self, tick: CfdTick) -> Result<IndexTick, IndexError> {
        let now = chrono::Utc::now().timestamp_millis();
        if (now - tick.ts_ms) as u64 > self.max_staleness_ms {
            return Err(IndexError::Stale);
        }
        if let Some(prev) = self.last_px {
            let jump = ((tick.price - prev)/prev).abs();
            if jump > self.jump_pct {
                return Err(IndexError::Jump);
            }
        }
        self.buf.push_back(tick);
        self.prune(now);
        self.last_px = Some(tick.price);

        // spike suppression via rolling median applied as anchor on TWAP
        let twap = self.twap(now).ok_or(IndexError::NoData)?;
        let med  = self.median().unwrap_or(twap);
        let fused = 0.5*twap + 0.5*med;

        Ok(IndexTick {
            symbol: self.symbol.clone(),
            price: fused,
            expo: self.expo,
            ts_ms: now,
            source: "cfd",
            window_sec: self.twap_sec,
        })
    }
}

