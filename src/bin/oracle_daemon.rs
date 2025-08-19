// src/bin/oracle_daemon.rs
use std::sync::Arc;
use std::time::Duration;

use autonom::{
    config::OracleConfig,
    oracle::Oracle,
    providers::{
        cfd::{NinjasCfd, OwninjaCfd},
        CfdProvider,
    },
    publishing::StdoutPublisher,
    funding::FundingEngine,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg_path = std::env::args().skip_while(|a| a != "--config").nth(1)
        .unwrap_or_else(|| "config/oracle.toml".to_string());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(p) = args.next() {
                cfg_path = p;
            }
        }
    }

    // --- load OracleConfig from TOML; fall back to Default if missing/unparseable
    let cfg = match std::fs::read_to_string(&cfg_path) {
        Ok(s) => match toml::from_str::<OracleConfig>(&s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("CONFIG DESERIALIZE ERROR [{}]:\n{}", cfg_path, e);
                OracleConfig::default()
            }
        },
        Err(e) => {
            eprintln!("CONFIG READ ERROR [{}]:\n{}", cfg_path, e);
            OracleConfig::default()
        }
    };

    // --- publisher (no Arc; Publisher is implemented for the concrete type)
    let publisher = StdoutPublisher {};

    let ninjas = std::sync::Arc::new(NinjasCfd::from_env()?);
    let owninja = std::sync::Arc::new(OwninjaCfd); // keep your mock if you want diversity

    // --- CFD providers (add/remove as your project implements them)
    let cfd_providers: Vec<std::sync::Arc<dyn CfdProvider + Send + Sync>> =
        vec![ninjas, owninja];

    // --- funding engine (simple default; adjust if you expose config knobs)
    let funding_engine = FundingEngine::new(
        0.02,      // kappa: strength of mean-reversion toward the reference
        0.005,     // cap: max funding magnitude per interval (e.g., 0.5%)
        8 * 60 * 60, // interval_sec: typical 8h funding window
    );

    // NOTE: this matches your current Oracle::new signature:
    // Oracle::new(cfg, publisher, cfds, funding_engine)
    let mut oracle = Oracle::new(cfg, publisher, cfd_providers, funding_engine);

    // drive ticks at cfg.poll_ms (fallback 1000ms if unset/zero)
    let tick_ms = if oracle.cfg.poll_ms == 0 { 1000 } else { oracle.cfg.poll_ms };
    let mut ticker = tokio::time::interval(Duration::from_millis(tick_ms as u64));

    // simple loop with Ctrl-C shutdown
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                oracle.tick_once().await;
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!("received Ctrl-C, exiting");
                break;
            }
        }
    }

    Ok(())
}
