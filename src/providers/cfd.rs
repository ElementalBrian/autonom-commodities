use crate::providers::CfdProvider;
use crate::types::{CfdQuote, CfdSource};
use anyhow::Error;
use rand::Rng;
use std::sync::{Mutex, OnceLock};

/// Simple shared state for a random-walk price per provider
static NINJAS_PX: OnceLock<Mutex<f64>> = OnceLock::new();
static OWNINJA_PX: OnceLock<Mutex<f64>> = OnceLock::new();

pub struct NinjasCfd;
pub struct OwninjaCfd;

#[async_trait::async_trait]
impl CfdProvider for NinjasCfd {
    fn name(&self) -> &'static str { "ninjas" }

    async fn latest(&self, _symbol: &str) -> Result<CfdQuote, Error> {
        let now = chrono::Utc::now().timestamp_millis();
        let px = {
            let m = NINJAS_PX.get_or_init(|| Mutex::new(0.905));
            let mut p = m.lock().unwrap();
            // ~±5 bps shock
            let shock: f64 = rand::thread_rng().gen_range(-0.0005..0.0005);
            *p = (*p * (1.0 + shock)).max(0.1);
            *p
        };
        Ok(CfdQuote { src: CfdSource::Ninjas, price: px, ts_ms: now })
    }
}

#[async_trait::async_trait]
impl CfdProvider for OwninjaCfd {
    fn name(&self) -> &'static str { "owninja" }

    async fn latest(&self, _symbol: &str) -> Result<CfdQuote, Error> {
        let now = chrono::Utc::now().timestamp_millis();
        let px = {
            let m = OWNINJA_PX.get_or_init(|| Mutex::new(0.907));
            let mut p = m.lock().unwrap();
            // ~±6 bps shock with tiny bias
            let shock: f64 = rand::thread_rng().gen_range(-0.0006..0.0006) + 0.00002;
            *p = (*p * (1.0 + shock)).max(0.1);
            *p
        };
        Ok(CfdQuote { src: CfdSource::Owninja, price: px, ts_ms: now })
    }
}
