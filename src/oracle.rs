// src/oracle.rs
use chrono::Utc;
use futures::future::join_all;
use std::sync::Arc;

use crate::config::OracleConfig;
use crate::funding::{Ema, FundingEngine};
use crate::index::cfd_consensus::CfdConsensus;
use crate::providers::CfdProvider;
use crate::publishing::Publisher;
use crate::types::{CfdQuote, IndexTick};

#[derive(Debug, Clone)]
struct CircuitBreaker {
    per_min_threshold: f64,
    last_anchor_price: Option<f64>,
    last_anchor_ms: Option<i64>,
}
impl CircuitBreaker {
    fn new(per_min_threshold: f64) -> Self {
        Self { per_min_threshold, last_anchor_price: None, last_anchor_ms: None }
    }
    fn tripped(&mut self, px: f64, ts_ms: i64) -> bool {
        match (self.last_anchor_price, self.last_anchor_ms) {
            (Some(base_px), Some(base_ts)) => {
                let dt_ms = (ts_ms - base_ts).max(1) as f64;
                let dt_ratio = 60_000.0 / dt_ms; // normalize to per-minute
                let change = (px / base_px - 1.0).abs() * dt_ratio;
                if change > self.per_min_threshold {
                    return true;
                }
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

pub struct Oracle<Pu>
where
    Pu: Publisher + Send + Sync + 'static,
{
    pub cfg: OracleConfig,
    pub publisher: Pu,
    pub cfds: Vec<Arc<dyn CfdProvider + Send + Sync>>,
    pub name: String,
    pub last_good_mark: Option<IndexTick>,
    pub funding_ref_ema: Ema,
    pub funding_engine: FundingEngine,
    cb: CircuitBreaker,
}

impl<Pu> Oracle<Pu>
where
    Pu: Publisher + Send + Sync + 'static,
{
    pub fn new(
        cfg: OracleConfig,
        publisher: Pu,
        cfds: Vec<Arc<dyn CfdProvider + Send + Sync>>,
        funding_engine: FundingEngine,
    ) -> Self {
        Self {
            cb: CircuitBreaker::new(0.07),
            funding_ref_ema: Ema::new(0.005),
            cfg,
            publisher,
            cfds,
            name: String::new(),
            last_good_mark: None,
            funding_engine,
        }
    }

    pub async fn tick_once(&mut self) {
        if !self.hours_ok() {
            return;
        }

        let (quotes, _attempted) = self.collect_cfd_quotes().await;

        // Staleness gate (~3×tau by default)
        let now = Utc::now().timestamp_millis();
        let max_stale_ms = self.derived_staleness_ms();
        let fresh: Vec<CfdQuote> = quotes
            .into_iter()
            .filter(|q| (now - q.ts_ms).unsigned_abs() <= max_stale_ms)
            .collect();

        if fresh.len() < self.cfg.cfd_min_fresh.max(1) {
            return;
        }

        // Robust consensus (4-arg constructor)
        let builder = CfdConsensus::new(
            self.cfg.symbol.clone(),
            self.cfg.expo,
            self.cfg.cfd_tau_ms,
            self.cfg.cfd_mad_k,
        );

        let (mut mark, stats) = match builder.build(&fresh) {
            Ok(x) => x,
            Err(_) => return,
        };

        // Optional dispersion check (soft)
        let _too_wide = stats.spread_bps > self.cfg.cfd_dispersion_bps_max;

        // Per-tick step clamp vs last good mark
        if let Some(prev) = &self.last_good_mark {
            let step = self.cfg.max_step_per_tick.max(0.0005);
            let lo = prev.price * (1.0 - step);
            let hi = prev.price * (1.0 + step);
            if mark.price < lo {
                mark.price = lo;
            } else if mark.price > hi {
                mark.price = hi;
            }
        }

        // Circuit breaker
        if self.cb.tripped(mark.price, mark.ts_ms) {
            if let Some(good) = &self.last_good_mark {
                mark = good.clone();
            } else {
                return;
            }
        } else {
            self.last_good_mark = Some(mark.clone());
        }

        // Publish mark
        if let Err(e) = self.publisher.publish_index(mark.clone()).await {
            tracing::warn!("publish_index failed: {e:?}");
        }

        // Funding vs slow EMA reference
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
    }

    fn derived_staleness_ms(&self) -> u64 {
        let three_tau = self.cfg.cfd_tau_ms.saturating_mul(3);
        three_tau.clamp(15_000, 120_000)
    }

    fn hours_ok(&self) -> bool {
        match self.cfg.hours_guard.as_str() {
            "off" => true,
            "vendor" => true, // CFDs ~23x5; permissive guard
            _ => true,        // TODO: wire CME calendar if desired
        }
    }

    async fn collect_cfd_quotes(&self) -> (Vec<CfdQuote>, usize) {
        let now = Utc::now().timestamp_millis();

        let futs = self
            .cfds
            .iter()
            .map(|prov| async move { prov.latest(&self.cfg.symbol).await });

        let results = join_all(futs).await;
        let attempted = results.len();
        let mut out = Vec::with_capacity(attempted);

        for res in results {
            match res {
                Ok(mut q) => {
                    if !q.price.is_finite() || q.price <= 0.0 {
                        continue;
                    }
                    // clamp far-future timestamps
                    if (q.ts_ms - now) > 2_000 {
                        q.ts_ms = now;
                    }
                    out.push(q);
                }
                Err(err) => {
                    tracing::debug!("CFD provider error: {:?}", err);
                }
            }
        }
        (out, attempted)
    }
}
