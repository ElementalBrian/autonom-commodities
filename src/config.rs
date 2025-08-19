// src/config.rs
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct OracleConfig {
    pub symbol: String,             // "LH"
    pub expo: i8,                   // -8
    pub poll_ms: u64,               // 500-1500ms typical (ingestion loop)
    pub cfd_twap_sec: u32,          // 30-60s
    pub cfd_median_sec: u32,        // 300-600s (spike suppression)
    pub cfd_max_staleness_ms: u64,  // 90_000
    pub cfd_jump_pct: f64,          // 0.05 (5%) reject rule outside roll
    pub cmf_target_days: f64,       // 30.0
    pub roll_hike_im_pct: f64,      // 0.25..0.5 applied downstream (advisory)
    pub funding_kappa: f64,         // 0.5
    pub funding_cap: f64,           // 0.004 per interval
    pub funding_interval_sec: u32,  // 8*3600
    pub trading_hours_only: bool,   // true
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            symbol: "LH".into(),
            expo: -8,
            poll_ms: 750,
            cfd_twap_sec: 30,
            cfd_median_sec: 600,
            cfd_max_staleness_ms: 90_000,
            cfd_jump_pct: 0.05,
            cmf_target_days: 30.0,
            roll_hike_im_pct: 0.3,
            funding_kappa: 0.5,
            funding_cap: 0.004,
            funding_interval_sec: 8 * 3600,
            trading_hours_only: true,
        }
    }
}

#[inline]
pub fn ms(d: u64) -> std::time::Duration { Duration::from_millis(d) }

#[derive(Debug, Clone, Deserialize)]
pub struct OracleConfig {
    // ...existing...
    pub mode_cfd_only: bool,         // ← set true when you have no CME
    pub cfd_min_fresh: usize,        // min providers needed (≥2 ideal)
    pub cfd_tau_ms: u64,             // freshness decay for weighting (e.g. 20_000)
    pub cfd_mad_k: f64,              // MAD threshold for outliers (e.g. 6.0)
    pub cfd_dispersion_bps_max: u32, // if providers disagree > X bps, mark "degraded"
    pub hours_guard: String,         // "cme" | "vendor" | "off"
    pub max_step_per_tick: f64,      // e.g., 0.02 = 2% clamp vs last mark
}

impl Default for OracleConfig {
    fn default() -> Self {
        let mut c = Self {
            // ...existing defaults...
            mode_cfd_only: false,
            cfd_min_fresh: 2,
            cfd_tau_ms: 20_000,
            cfd_mad_k: 6.0,
            cfd_dispersion_bps_max: 80,
            hours_guard: "cme".into(),
            max_step_per_tick: 0.02,
        };
        c
    }
}

