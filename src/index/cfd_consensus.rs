use crate::types::{IndexTick, CfdQuote, ConsensusStats};
use crate::index::IndexError;

/// Robust consensus over multiple CFD providers:
/// 1) median anchor
/// 2) MAD outlier rejection
/// 3) freshness-weighted mean around the median
pub struct CfdConsensus {
    pub symbol: String,
    pub expo: i8,
    pub tau_ms: u64,
    pub mad_k: f64, // keep quotes within +/- mad_k * MAD around median
}

impl CfdConsensus {
    pub fn new(symbol: String, expo: i8, tau_ms: u64, mad_k: f64) -> Self {
        Self { symbol, expo, tau_ms, mad_k }
    }

    /// Returns median price if at least one price present.
    pub fn fuse(&self, mut prices: Vec<f64>) -> Option<f64> {
        if prices.is_empty() {
            return None;
        }
        prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = prices.len() / 2;
        if prices.len() % 2 == 1 {
            Some(prices[mid])
        } else {
            Some((prices[mid - 1] + prices[mid]) / 2.0)
        }
    }

    fn median(prices: &mut [f64]) -> f64 {
        prices.sort_by(|a,b| a.partial_cmp(b).unwrap());
        prices[prices.len()/2]
    }

    fn mad(values: &[f64], med: f64) -> f64 {
        let mut devs: Vec<f64> = values.iter().map(|v| (v - med).abs()).collect();
        devs.sort_by(|a,b| a.partial_cmp(b).unwrap());
        let m = devs[devs.len()/2];
        // Consistent MAD (≈ std) factor for normal: 1.4826
        1.4826 * m.max(1e-9)
    }

    pub fn build(&self, quotes: &[CfdQuote]) -> Result<(IndexTick, ConsensusStats), IndexError> {
        if quotes.is_empty() { return Err(IndexError::NoData); }
        let now = chrono::Utc::now().timestamp_millis();

        // anchor on median
        let mut ps: Vec<f64> = quotes.iter().map(|q| q.price).collect();
        let med = Self::median(&mut ps);
        let mad = Self::mad(&ps, med);

        // outlier filter
        let band = self.mad_k * mad;
        let mut kept = Vec::new();
        let mut minp = f64::INFINITY; let mut maxp = f64::NEG_INFINITY;
        for q in quotes {
            if (q.price - med).abs() <= band {
                kept.push(*q);
                if q.price < minp { minp = q.price; }
                if q.price > maxp { maxp = q.price; }
            }
        }
        if kept.is_empty() { return Err(IndexError::NoData); }

        // freshness-weighted average around median
        let mut num = 0.0; let mut den = 0.0;
        for q in &kept {
            let age = (now - q.ts_ms).unsigned_abs() as f64;
            let w = f64::exp(-age / self.tau_ms as f64);
            // also damp weights far from median (gentle)
            let dev = ((q.price - med).abs() / (mad + 1e-9)).min(10.0);
            let w2 = w * f64::exp(-0.15 * dev);
            num += w2 * q.price;
            den += w2;
        }
        if den <= 0.0 { return Err(IndexError::NoData); }
        let fused = num / den;

        let spread_bps = (((maxp - minp) / med).abs() * 10_000.0).round() as u32;
        let confidence = {
            let n = kept.len() as f32 / (quotes.len().max(1) as f32);
            let tight = 1.0_f32 / (1.0 + (spread_bps as f32 / 50.0));
            (n * tight).min(1.0)
        };

        let tick = IndexTick {
            symbol: self.symbol.clone(),
            price: fused,
            expo: self.expo,
            ts_ms: now,
            source: "cfd-consensus",
            window_sec: 0,
        };
        let stats = ConsensusStats {
            n_fresh: quotes.len(),
            n_used: kept.len(),
            n_dropped: quotes.len().saturating_sub(kept.len()),
            spread_bps,
            confidence,
        };
        Ok((tick, stats))
    }
}

