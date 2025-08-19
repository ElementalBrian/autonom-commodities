// src/index/cfd.rs

use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CfdTick};
use std::collections::VecDeque;

/// Keep ticks for the last 20 seconds (in milliseconds).
const MAX_AGE_MS: i64 = 20_000;

pub struct CfdIndexBuilder {
    buf: VecDeque<CfdTick>,
    last_px: Option<f64>,
}

impl CfdIndexBuilder {
    pub fn new() -> Self {
        Self {
            buf: VecDeque::with_capacity(64),
            last_px: None,
        }
    }

    fn prune(&mut self, now_ms: i64) {
        while let Some(front) = self.buf.front() {
            // Pop anything older than MAX_AGE_MS
            if now_ms - front.ts_ms > MAX_AGE_MS {
                self.buf.pop_front();
            } else {
                break;
            }
        }
    }
}

impl IndexBuilder<CfdTick> for CfdIndexBuilder {
    fn build(&mut self, tick: CfdTick) -> Result<IndexTick, IndexError> {
        // Extract fields we need before moving `tick`
        let now_ms = tick.ts_ms;
        let px = tick.price;

        // Ingest and prune old samples
        self.buf.push_back(tick);
        self.prune(now_ms);

        if self.buf.is_empty() {
            return Err(IndexError::NotEnoughData);
        }

        // Simple rolling TWAP over the kept window
        let (sum, n) = self
            .buf
            .iter()
            .fold((0.0, 0usize), |(s, c), t| (s + t.price, c + 1));
        let twap = sum / n as f64;

        self.last_px = Some(px);

        Ok(IndexTick {
            symbol: "CFD".to_string(),          // set by caller if you want per-asset symbols
            price: twap,                        // windowed average (not just last)
            expo: -8,                           // typical commodity exponent
            ts_ms: now_ms,
            source: "cfd",                      // static str fits &'static str
            window_sec: (MAX_AGE_MS / 1000) as u32,
        })
    }
}
