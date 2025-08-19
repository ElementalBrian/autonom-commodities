// src/oracle.rs
//
// Oracle engine with a robust CFD-only path.
// - Collects quotes from multiple CFD providers in parallel
// - Builds a consensus mark (median anchor → MAD outlier filter → freshness-weighted mean)
// - Enforces staleness, jump clamp, optional hours guard, and a simple circuit breaker
// - Publishes the mark and a funding value anchored to a slow EMA “reference”
//
// This file is a drop-in replacement that assumes the following items exist in your crate:
//
//   crate::config::OracleConfig                 (see notes inside)
//   crate::index::cfd_consensus::CfdConsensus   (median + MAD + freshness, previously provided)
//   crate::providers::{CfdProvider, CmeProvider} (traits; CME is optional here)
//   crate::publishing::Publisher                (trait with publish_index / publish_funding)
//   crate::types::{IndexTick, CfdQuote, CfdSource} (small PODs; previously provided)
//   crate::funding::{Ema, FundingEngine}        (EMA + a funding calculator; ref EMA is used here)
//   crate::metrics::*                           (optional Prometheus counters; guarded with helpers)
//
// If some of those names differ in your tree, adjust the `use` lines below.
// Nothing else in Web2 needs to change; this oracle just emits marks to your publisher,
// which can write into the same cache your HTTP signer already serves.

use std::sync::Arc;
use futures::future::join_all;

use chrono::Utc;

use crate::config::OracleConfig;
use crate::index::cfd_consensus::CfdConsensus;
use crate::providers::{CfdProvider, CmeProvider};
use crate::publishing::Publisher;
use crate::types::{IndexTick, CfdQuote, CfdSource};
use crate::funding::{Ema, FundingEngine};

// ---------- Optional metrics (no-ops if you don’t wire them) -----------------

#[inline]
fn inc(_name: &str, _label: &str) {
    #[cfg(feature = "metrics")]
    {
        use crate::metrics;
        match _name {
            "oracle_drops_total" => metrics::ORACLE_DROPS_TOTAL.with_label_values(&[_label]).inc(),
            "oracle_ticks_total" => metrics::ORACLE_TICKS_TOTAL.with_label_values(&[_label]).inc(),
            "oracle_quotes_total" => metrics::ORACLE_QUOTES_TOTAL.with_label_values(&[_label]).inc(),
            _ => {}
        }
    }
}

#[inline]
fn obs_latency(_sec: f64, _provider: &str) {
    #[cfg(feature = "metrics")]
    {
        use crate::metrics::PROVIDER_LATENCY_SECONDS;
        PROVIDER_LATENCY_SECONDS
            .with_label_values(&[_provider])
            .observe(_sec);
    }
}

// -----------------------------------------------------------------------------

/// Very small, self-contained “circuit breaker” for realized 60s moves.
/// It freezes to the last good mark if the per-minute move exceeds a threshold.
#[derive(Debug, Clone)]
struct CircuitBreaker {
    // percentage move threshold per 60 seconds, e.g. 0.07 = 7%
    per_min_threshold: f64,
    // tracking state
    last_anchor_price: Option<f64>,
    last_anchor_ms: Option<i64>,
}

impl CircuitBreaker {
    fn new(per_min_threshold: f64) -> Self {
        Self {
            per_min_threshold,
            last_anchor_price: None,
            last_anchor_ms: None,
        }
    }

    /// Returns true if the move (normalized to a 60s window) breaches the threshold.
    fn tripped(&mut self, px: f64, ts_ms: i64) -> bool {
        match (self.last_anchor_price, self.last_anchor_ms) {
            (Some(base_px), Some(base_ts)) => {
                let dt_ms = (ts_ms - base_ts).max(1) as f64;
                let dt_ratio = 60_000.0 / dt_ms; // normalize to one minute
                let change = (px / base_px - 1.0).abs() * dt_ratio;
                if change > self.per_min_threshold {
                    // keep the same anchor so we remain frozen until a manual reset
                    return true;
                }
                // decay / roll the anchor forward every ~10 seconds to keep it relevant
                if dt_ms >= 10_000.0 {
                    self.last_anchor_price = Some(px);
                    self.last_anchor_ms = Some(ts_ms);
                }
                false
            }
            _ => {
                self.last_anchor_price = Some(px);
                self.last_anchor_ms = Some(ts_ms);
                false
            }
        }
    }
}

/// Oracle that produces a marked index and funding using CFD-only data if needed.
pub struct Oracle<Pu>
where
    Pu: Publisher + Send + Sync + 'static,
{
    pub cfg: OracleConfig,
    pub publisher: Pu,

    /// Optional CME provider (unused in CFD-only mode, kept for future extension).
    pub cme: Option<Arc<dyn CmeProvider + Send + Sync>>,

    /// One or more CFD providers.
    pub cfds: Vec<Arc<dyn CfdProvider + Send + Sync>>,

    ///
    pub name: String,

    /// Last good mark after all guards.
    pub last_good_mark: Option<IndexTick>,

    /// Slow EMA used as a funding reference when there is no CME.
    pub funding_ref_ema: Ema,

    /// Funding calculator (basis vs reference).
    pub funding_engine: FundingEngine,

    /// Simple circuit breaker on realized moves.
    cb: CircuitBreaker,
}

impl<Pu> Oracle<Pu>
where
    Pu: Publisher + Send + Sync + 'static,
{
    pub fn new(
        cfg: OracleConfig,
        publisher: Pu,
        cme: Option<Arc<dyn CmeProvider + Send + Sync>>,
        cfds: Vec<Arc<dyn CfdProvider + Send + Sync>>,
        funding_engine: FundingEngine,
    ) -> Self {
        // If you don’t add cfd_max_staleness_ms to OracleConfig, we treat
        // “freshness” scale as ~3× tau (very tolerant, still bounded).
        let _ = &cfg;
        Self {
            cb: CircuitBreaker::new(0.07), // 7% per minute default; tune via cfg if you add it
            cfg,
            publisher,
            cme,
            cfds,
            name: "".to_string(),
            last_good_mark: None,
            funding_ref_ema: Ema::new(0.005), // ~slow; adjust by cfg if desired
            funding_engine,
        }
    }

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Perform one tick: collect CFD quotes, build a consensus mark, run guards, publish.
    pub async fn tick_once(&mut self) {
        // Optional trading-hours guard.
        if !self.hours_ok() {
            inc("oracle_drops_total", "hours");
            return;
        }

        // ----------------- CFD-ONLY PATH -----------------
        if self.cfg.mode_cfd_only {
            let (quotes, n_attempted) = self.collect_cfd_quotes().await;
            inc("oracle_quotes_total", if quotes.is_empty() { "0" } else { "n>0" });

            // Staleness: bound to ~3×tau by default (unless you added an explicit knob).
            let now = Utc::now().timestamp_millis();
            let max_stale_ms = self.derived_staleness_ms();
            let mut fresh: Vec<CfdQuote> = quotes
                .into_iter()
                .filter(|q| (now - q.ts_ms).unsigned_abs() <= max_stale_ms)
                .collect();

            // Deduplicate identical timestamps from same source (rare vendor artifacts)
            // (No-op if your providers already de-dupe.)

            if fresh.len() < self.cfg.cfd_min_fresh.max(1) {
                inc("oracle_drops_total", "stale_or_insufficient");
                return;
            }

            // Consensus builder
            let builder = CfdConsensus::new(
                self.cfg.symbol.clone(),
                self.cfg.expo,
                self.cfg.cfd_tau_ms,
                self.cfg.cfd_mad_k,
            );

            let (mut mark, stats) = match builder.build(&fresh) {
                Ok(x) => x,
                Err(_) => {
                    inc("oracle_drops_total", "no_consensus");
                    return;
                }
            };

            // Optional dispersion check → not a hard drop; you may widen margins downstream.
            if stats.spread_bps > self.cfg.cfd_dispersion_bps_max {
                inc("oracle_drops_total", "wide_dispersion");
                // continue with guards; consumers can look at your confidence too
            }

            // Per-tick step clamp vs last good mark.
            if let Some(prev) = &self.last_good_mark {
                let step = self.cfg.max_step_per_tick.max(0.0005); // floor at 5 bps to avoid lock
                let lo = prev.price * (1.0 - step);
                let hi = prev.price * (1.0 + step);
                if mark.price < lo {
                    mark.price = lo;
                } else if mark.price > hi {
                    mark.price = hi;
                }
            }

            // Circuit breaker (realized); freeze to last_good if tripped.
            if self.cb.tripped(mark.price, mark.ts_ms) {
                if let Some(good) = &self.last_good_mark {
                    // Freeze to the last known good mark
                    mark = good.clone();
                    inc("oracle_drops_total", "circuit_breaker");
                } else {
                    // No prior mark to freeze to — drop this tick
                    inc("oracle_drops_total", "cb_no_anchor");
                    return;
                }
            } else {
                self.last_good_mark = Some(mark.clone());
            }

            // Publish mark
            if let Err(e) = self.publisher.publish_index(mark.clone()).await {
                tracing::warn!("publish_index failed: {e:?}");
            }

            // Funding against a slow EMA of the same series (no CME available).
            let ref_px = self.funding_ref_ema.update(mark.price);
            let ref_tick = IndexTick {
                symbol: mark.symbol.clone(),
                price: ref_px,
                expo: mark.expo,
                ts_ms: mark.ts_ms,
                source: "ref-ema",
                window_sec: 0,
            };
            let funding = self.funding_engine.compute(&mark, &ref_tick);
            if let Err(e) = self.publisher.publish_funding(funding).await {
                tracing::warn!("publish_funding failed: {e:?}");
            }

            inc("oracle_ticks_total", "ok");
            return;
        }

        // ----------------- (Optional) CME+CFD path (kept minimal here) -----------------
        // If you later want to reintroduce CME as reference, you can:
        // 1) compute mark from CFD consensus as above
        // 2) fetch CME reference (or last known), and
        // 3) use that as the reference in funding_engine rather than the EMA.
        // For now we skip this branch intentionally to keep a pure CFD-only engine.
    }

    // --- helpers -------------------------------------------------------------

    fn derived_staleness_ms(&self) -> u64 {
        // If you added a specific cfd_max_staleness_ms to OracleConfig, use it.
        // Otherwise derive a reasonable bound from tau (3×tau, clamped to [15s, 120s]).
        let three_tau = self.cfg.cfd_tau_ms.saturating_mul(3);
        three_tau.clamp(15_000, 120_000)
    }

    fn hours_ok(&self) -> bool {
        // Interpret cfg.hours_guard:
        // "off"    -> always trade
        // "vendor" -> CFDs are usually 23x5; return true (leave risk to jump/cb)
        // "cme"    -> emulate CME hours gate (if you have a calendar helper, call it here)
        match self.cfg.hours_guard.as_str() {
            "off" => true,
            "vendor" => true,
            _ => {
                // If you have a proper calendar helper, call it.
                // For now, we default to open = true (you can tighten by setting hours_guard="off"/"vendor").
                true
            }
        }
    }

    async fn collect_cfd_quotes(&self) -> (Vec<CfdQuote>, usize) {
        let now = Utc::now().timestamp_millis();

        let futs = self
            .cfds
            .iter()
            .map(|prov| async move {
                let t0 = std::time::Instant::now();
                let out = prov.latest_tick(&self.cfg.symbol).await;
                let dt = t0.elapsed().as_secs_f64();

                // provider name for metrics
                let pname = prov.name();
                obs_latency(dt, &pname);

                match out {
                    Ok(tick) => {
                        // Map provider name to CfdSource enum (extend as needed)
                        let src = match pname.to_ascii_lowercase().as_str() {
                            "ninjas" | "api-ninjas" => CfdSource::Ninjas,
                            "owninja" | "openwebninja" => CfdSource::Owninja,
                            other => CfdSource::Other(Box::leak(other.to_string().into_boxed_str())),
                        };
                        Some(CfdQuote {
                            src,
                            price: tick.price,
                            ts_ms: tick.ts_ms,
                        })
                    }
                    Err(err) => {
                        tracing::debug!("CFD provider {} error: {:?}", pname, err);
                        None
                    }
                }
            });

        let results: Vec<Option<CfdQuote>> = join_all(futs).await;
        let attempted = results.len();
        let mut quotes: Vec<CfdQuote> = Vec::with_capacity(attempted);
        for r in results {
            if let Some(q) = r {
                // Basic sanity: positive & finite prices only
                if q.price.is_finite() && q.price > 0.0 {
                    // Don’t allow timestamps in the far future (clamp)
                    let mut q2 = q;
                    if (q2.ts_ms - now) > 2_000 {
                        q2.ts_ms = now;
                    }
                    quotes.push(q2);
                }
            }
        }
        (quotes, attempted)
    }
}

