// src/bin/oracle_daemon.rs
use commodities_oracle::{
    config::OracleConfig,
    oracle::Oracle,
    publishing::StdoutPublisher,
    providers::{cfd::NinjasCfd, cme::DummyCme},
};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use autonom::oracle::Oracle;
use autonom::publishing::StdoutPublisher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cfg = OracleConfig::default();

    let publisher = Arc::new(StdoutPublisher);
    let cme = Arc::new(DummyCme);
    let cfd = Arc::new(NinjasCfd {
        http: reqwest::Client::new(),
        api_key: std::env::var("API_NINJAS_KEY").unwrap_or_default(),
    });

    let oracle = Oracle::new(cfg, publisher, cme, cfd);
    oracle.run().await;
    Ok(())
}

