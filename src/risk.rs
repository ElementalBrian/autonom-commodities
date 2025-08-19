// src/risk.rs
use crate::index::IndexError;
use chrono::{Datelike, Timelike};

#[derive(Debug, Clone)]
pub struct RiskSwitches {
    pub circuit_breaker: bool,
    pub roll_window: bool,
    pub hours_open: bool,
}

pub struct RiskEngine {
    pub max_delta_pct_60s: f64, // 0.08 (8%)
    last_px: Option<(f64, i64)>,
}

impl RiskEngine {
    pub fn new(max_delta_pct_60s: f64) -> Self { Self { max_delta_pct_60s, last_px: None } }

    pub fn trading_hours_open(&self, tz_offset_hours: i32) -> bool {
        // Simplified Mon–Fri 09:00–14:00 local (replace with CME calendar)
        let now = chrono::Utc::now() + chrono::Duration::hours(tz_offset_hours as i64);
        let wd = now.weekday().number_from_monday();
        let h = now.hour() as i32;
        (1..=5).contains(&wd) && (9..=14).contains(&h)
    }

    pub fn eval_circuit_breaker(&mut self, last_good: Option<(f64, i64)>, new_px: f64, now_ms: i64) -> bool {
        let base = last_good.or(self.last_px).unwrap_or((new_px, now_ms));
        let dt = (now_ms - base.1).max(1) as f64 / 1000.0;
        let dv = ((new_px - base.0)/base.0).abs();
        self.last_px = Some((new_px, now_ms));
        dt <= 60.0 && dv > self.max_delta_pct_60s
    }

    pub fn map_index_error(&self, e: IndexError) -> &'static str {
        match e {
            IndexError::Stale => "stale",
            IndexError::Jump => "jump",
            IndexError::Hours => "hours",
            IndexError::NoData => "nodata",
            IndexError::Internal(_) => "internal",
        }
    }
}

