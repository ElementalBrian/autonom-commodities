// src/risk.rs
use crate::index::IndexError;
use chrono::{Datelike, Timelike};

#[derive(Debug, Clone, Copy)]
pub struct RiskSwitches {
    pub circuit_breaker: bool,
    pub roll_window: bool,
    pub hours_open: bool,
}

pub struct RiskEngine {
    /// Maximum allowed absolute percent move over a 60s lookback (e.g., 0.08 = 8%)
    pub max_delta_pct_60s: f64,
    /// (last_price, last_ts_ms)
    last_px: Option<(f64, i64)>,
}

impl RiskEngine {
    pub fn new(max_delta_pct_60s: f64) -> Self {
        Self { max_delta_pct_60s, last_px: None }
    }

    /// Simplified trading-hours check: Mon–Fri 09:00–14:00 in a caller-supplied local offset.
    /// Replace with a real exchange calendar when ready.
    pub fn trading_hours_open(&self, tz_offset_hours: i32) -> bool {
        let now = chrono::Utc::now() + chrono::Duration::hours(tz_offset_hours as i64);
        let wd = now.weekday().number_from_monday(); // 1..=5
        let h = now.hour() as i32;                   // 0..23
        (1..=5).contains(&wd) && (9..=14).contains(&h)
    }

    /// One-step circuit breaker: trips if an absolute return over the last ~60s
    /// exceeds `max_delta_pct_60s`. You can pass an optional authoritative
    /// `(price, ts_ms)` as `last_good` (e.g., last published fused price).
    pub fn eval_circuit_breaker(
        &mut self,
        last_good: Option<(f64, i64)>,
        new_px: f64,
        now_ms: i64,
    ) -> bool {
        let base = last_good
            .or(self.last_px)
            .unwrap_or((new_px, now_ms));
        let dt = (now_ms - base.1).max(1) as f64 / 1000.0; // seconds
        let dv = ((new_px - base.0) / base.0).abs();
        self.last_px = Some((new_px, now_ms));
        dt <= 60.0 && dv > self.max_delta_pct_60s
    }

    /// Map index errors to short, user-facing risk codes.
    pub fn map_index_error(&self, e: IndexError) -> &'static str {
        match e {
            IndexError::NotEnoughData   => "nodata",
            IndexError::StaleInput      => "stale",
            IndexError::InvalidInput(_) => "invalid",
            IndexError::Internal(_)     => "internal",
        }
    }

    /// Convenience helper to compute all risk switches at once.
    /// - `tz_offset_hours`: local offset for hours gating
    /// - `last_good`: optional authoritative last price
    /// - `maybe_new_px`: optional new tick to test breaker (price, ts_ms)
    /// - `roll_active`: set by your roll scheduler
    pub fn compute_switches(
        &mut self,
        tz_offset_hours: i32,
        last_good: Option<(f64, i64)>,
        maybe_new_px: Option<(f64, i64)>,
        roll_active: bool,
    ) -> RiskSwitches {
        let hours_open = self.trading_hours_open(tz_offset_hours);
        let circuit_breaker = if let Some((px, ts)) = maybe_new_px {
            self.eval_circuit_breaker(last_good, px, ts)
        } else {
            false
        };
        RiskSwitches {
            circuit_breaker,
            roll_window: roll_active,
            hours_open,
        }
    }
}
