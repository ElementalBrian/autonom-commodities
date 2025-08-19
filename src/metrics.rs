// src/metrics.rs
use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, register_histogram_vec, IntCounterVec, HistogramVec};

pub static TICKS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "oracle_ticks_total", "Ticks processed", &["source"] // cmf|cfd|mark
    ).unwrap()
});

pub static DROPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "oracle_drops_total", "Ticks rejected", &["reason"] // stale|jump|hours|cb
    ).unwrap()
});

pub static BUILD_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "oracle_build_latency_seconds",
        "Index build latency",
        &["path"], // cmf|cfd|mark
        vec![0.01, 0.02, 0.05, 0.1, 0.2, 0.35, 0.5]
    ).unwrap()
});

