// src/index/cfd.rs

use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CfdTick};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

const MAX_AGE: Duration = Duration::from_secs(20);

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

    fn prune(&mut self, now: SystemTime) {
        while let Some(front) = self.buf.front() {
            if now
                .duration_since(front.ts)
                .map(|d| d > MAX_AGE)
                .unwrap_or(false)
            {
                self.buf.pop_front();
            } else {
                break;
            }
        }
    }
}

impl IndexBuilder<CfdTick> for CfdIndexBuilder {
    fn build(&mut self, tick: CfdTick) -> Result<IndexTick, IndexError> {
        let now = tick.ts;
        let px = tick.price; // copy out what we need before moving `tick`

        self.buf.push_back(tick);
        self.prune(now);

        if self.buf.is_empty() {
            return Err(IndexError::NotEnoughData);
        }

        // Simple “index” = last price for now (placeholder)
        self.last_px = Some(px);

        Ok(IndexTick {
            ts: now,
            price: px,
            // add fields as needed by your IndexTick
        })
    }
}
