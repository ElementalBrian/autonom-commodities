// src/metrics.rs

#[cfg(feature = "metrics")]
mod imp {
    use once_cell::sync::Lazy;
    use prometheus::{
        register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
    };

    pub static REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
        register_int_counter_vec!(
            "requests_total",
            "HTTP requests received by endpoint",
            &["endpoint"]
        )
            .unwrap()
    });

    pub static RESPONSES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
        register_int_counter_vec!(
            "responses_total",
            "HTTP responses sent",
            &["endpoint", "status"]
        )
            .unwrap()
    });

    pub static PROVIDER_ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
        register_int_counter_vec!(
            "provider_errors_total",
            "Errors returned by providers",
            &["provider", "reason"]
        )
            .unwrap()
    });

    pub static PROVIDER_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
        register_histogram_vec!(
            "provider_latency_seconds",
            "Latency of provider fetches",
            &["provider"],
            vec![0.02, 0.05, 0.1, 0.2, 0.35, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0]
        )
            .unwrap()
    });

    pub fn init() {}
}

#[cfg(not(feature = "metrics"))]
mod imp {
    // No-op stand-ins so the rest of the code doesn't need #[cfg] everywhere.
    pub struct IntCounterVec;
    pub struct HistogramVec;
    impl IntCounterVec {
        pub fn with_label_values(&self, _labels: &[&str]) -> &Self {
            self
        }
        pub fn inc(&self) {}
    }
    impl HistogramVec {
        pub fn with_label_values(&self, _labels: &[&str]) -> &Self {
            self
        }
        pub fn observe(&self, _v: f64) {}
    }

    pub static REQUESTS_TOTAL: IntCounterVec = IntCounterVec;
    pub static RESPONSES_TOTAL: IntCounterVec = IntCounterVec;
    pub static PROVIDER_ERRORS_TOTAL: IntCounterVec = IntCounterVec;
    pub static PROVIDER_LATENCY_SECONDS: HistogramVec = HistogramVec;

    pub fn init() {}
}

// Re-export unified API
pub use imp::*;
