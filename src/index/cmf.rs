// src/index/cmf.rs

use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CmfInputs};

/// Constant-Maturity Futures (CMF) over two adjacent expiries.
/// We linearly interpolate prices in *time to expiry* space to hit `target_days`.
pub struct CmfIndexBuilder {
    pub symbol: String,
    pub expo: i8,          // e.g. -8 for 1e-8 scaling
}

impl CmfIndexBuilder {
    pub fn new<S: Into<String>>(symbol: S, expo: i8) -> Self {
        Self { symbol: symbol.into(), expo }
    }

    /// Days between `now_ms` and `future_ms` (non-negative, in fractional days).
    #[inline]
    fn days_to(now_ms: i64, future_ms: i64) -> f64 {
        let dt_ms = future_ms.saturating_sub(now_ms).max(0) as f64;
        dt_ms / 86_400_000.0
    }
}

impl IndexBuilder<CmfInputs> for CmfIndexBuilder {
    fn build(&mut self, tick: CmfInputs) -> Result<IndexTick, IndexError> {
        // Use wall clock for time-to-expiry math
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Compute time-to-expiry (days) for the two legs
        let mut d1 = Self::days_to(now_ms, tick.f1.expiry_ts_ms);
        let mut d2 = Self::days_to(now_ms, tick.f2.expiry_ts_ms);
        let mut p1 = tick.f1.price;
        let mut p2 = tick.f2.price;

        // Ensure d1 <= d2 by swapping if needed (expect front < next)
        if d1 > d2 {
            std::mem::swap(&mut d1, &mut d2);
            std::mem::swap(&mut p1, &mut p2);
        }

        // Sanity checks
        if !p1.is_finite() || !p2.is_finite() {
            return Err(IndexError::InvalidInput("non-finite price".into()));
        }
        if !d1.is_finite() || !d2.is_finite() || (d1 == 0.0 && d2 == 0.0) {
            return Err(IndexError::InvalidInput("invalid time-to-expiry".into()));
        }

        // Target constant maturity (in days)
        let tau = tick.target_days.max(0.0);

        // If both legs have (practically) the same maturity, just take the first price
        if (d2 - d1).abs() < 1e-9 {
            return Ok(IndexTick {
                symbol: self.symbol.clone(),
                price: p1,
                expo: self.expo,
                ts_ms: now_ms,
                source: "cmf",
                window_sec: 0,
            });
        }

        // Linear interpolation in maturity space:
        // w1 + w2 = 1; w1*d1 + w2*d2 = tau  =>  w2 = (tau - d1)/(d2 - d1), w1 = 1 - w2.
        // Clamp to [0,1] to avoid extrapolation (use nearest leg if outside).
        let mut w2 = (tau - d1) / (d2 - d1);
        if !w2.is_finite() {
            return Err(IndexError::InvalidInput("non-finite weight".into()));
        }
        w2 = w2.clamp(0.0, 1.0);
        let w1 = 1.0 - w2;

        let cmf_price = w1 * p1 + w2 * p2;

        Ok(IndexTick {
            symbol: self.symbol.clone(),
            price: cmf_price,
            expo: self.expo,
            ts_ms: now_ms,
            source: "cmf",
            window_sec: 0,
        })
    }
}
