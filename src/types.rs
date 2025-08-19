// src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PricePoint {
    pub price: f64,
    pub expo: i8,   // -8 for most commodities, -10 e.g. USDC
    pub ts_ms: i64, // unix ms
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexTick {
    pub symbol: String,        // e.g., "LH"
    pub price: f64,            // float form (will be scaled)
    pub expo: i8,              // -8 default
    pub ts_ms: i64,
    pub source: &'static str,  // "cmf" | "cfd" | "cfd-consensus" | "ref-ema" etc.
    pub window_sec: u32,       // TWAP period applied
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingUpdate {
    pub symbol: String,      // "LH-PERP"
    pub rate: f64,           // signed fraction per interval (e.g. 0.004 = 0.4%)
    pub interval_sec: u32,   // e.g. 8h
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct FuturesLeg {
    pub price: f64,
    pub ts_ms: i64,
    pub expiry_ts_ms: i64,  // contract expiry used for time weighting
}

#[derive(Debug, Clone, Copy)]
pub struct CmfInputs {
    pub f1: FuturesLeg,   // front month
    pub f2: FuturesLeg,   // next month
    pub target_days: f64, // e.g., 30d constant maturity
}

#[derive(Debug, Clone, Copy)]
pub struct CfdTick {
    pub price: f64,
    pub ts_ms: i64,
}

#[inline]
pub fn scale_by_expo(px: f64, expo: i8) -> Result<u64, &'static str> {
    if !px.is_finite() || px < 0.0 { return Err("invalid price"); }
    let factor = match expo {
        -8  => 100_000_000.0,
        -10 => 10_000_000_000.0,
        _   => return Err("unsupported expo"),
    };
    Ok((px * factor).round() as u64)
}

// ---- CFD quoting types ----

// Use owned String so serde derives are painless.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CfdSource { Ninjas, Owninja, Other(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfdQuote {
    pub src: CfdSource,
    pub price: f64,
    pub ts_ms: i64,
}

// Optional telemetry you can publish with a tick
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub n_fresh: usize,
    pub n_used: usize,
    pub n_dropped: usize,
    pub spread_bps: u32,   // (max-min)/median in bps
    pub confidence: f32,   // 0..1
}
