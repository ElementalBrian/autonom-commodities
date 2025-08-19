// src/config.rs
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct OracleConfig {
    pub symbol: String,
    pub expo: i8,
    #[serde(default = "d_poll_ms")]              pub poll_ms: u64,
    #[serde(default)]                            pub cfd_twap_sec: u32,
    #[serde(default)]                            pub cfd_median_sec: u32,
    #[serde(default = "d_stale_ms")]             pub cfd_max_staleness_ms: u64,
    #[serde(default = "d_jump_pct")]             pub cfd_jump_pct: f64,
    #[serde(default = "d_cmf_days")]             pub cmf_target_days: f64,
    #[serde(default = "d_roll_hike")]            pub roll_hike_im_pct: f64,
    #[serde(default = "d_funding_kappa")]        pub funding_kappa: f64,
    #[serde(default = "d_funding_cap")]          pub funding_cap: f64,
    #[serde(default = "d_funding_interval")]     pub funding_interval_sec: u32,
    #[serde(default)]                            pub trading_hours_only: bool,
    #[serde(default)]                            pub mode_cfd_only: bool,
    #[serde(default = "d_min_fresh")]            pub cfd_min_fresh: usize,
    #[serde(default = "d_tau_ms")]               pub cfd_tau_ms: u64,
    #[serde(default = "d_mad_k")]                pub cfd_mad_k: f64,
    #[serde(default = "d_dispersion_bps")]       pub cfd_dispersion_bps_max: u32,
    #[serde(default = "d_hours_guard")]          pub hours_guard: String,
    #[serde(default = "d_max_step")]             pub max_step_per_tick: f64,
}
fn d_poll_ms() -> u64 { 2000 }
fn d_stale_ms() -> u64 { 90_000 }
fn d_jump_pct() -> f64 { 0.05 }
fn d_cmf_days() -> f64 { 30.0 }
fn d_roll_hike() -> f64 { 0.25 }
fn d_funding_kappa() -> f64 { 0.5 }
fn d_funding_cap() -> f64 { 0.004 }
fn d_funding_interval() -> u32 { 8*3600 }
fn d_min_fresh() -> usize { 2 }
fn d_tau_ms() -> u64 { 8000 }
fn d_mad_k() -> f64 { 3.5 }
fn d_dispersion_bps() -> u32 { 35 }
fn d_hours_guard() -> String { "vendor".into() }
fn d_max_step() -> f64 { 0.01 }
#[inline]
pub fn ms(d: u64) -> std::time::Duration { Duration::from_millis(d) }

impl Default for OracleConfig {
    fn default() -> Self {
        let c = Self {
            symbol: "".to_string(),
            expo: 0,
            poll_ms: 0,
            cfd_twap_sec: 0,
            cfd_median_sec: 0,
            cfd_max_staleness_ms: 0,
            cfd_jump_pct: 0.0,
            cmf_target_days: 0.0,
            roll_hike_im_pct: 0.0,
            funding_kappa: 0.0,
            funding_cap: 0.0,
            funding_interval_sec: 0,
            trading_hours_only: false,
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

