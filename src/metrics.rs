use axum::{routing::get, Router};
use metrics::{counter, histogram, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;

pub fn init_metrics() {
    // Initialize Prometheus exporter
    PrometheusBuilder::new()
        .install()
        .expect("Failed to install Prometheus recorder");
    
    // Register metrics
    gauge!("rs_helper_up", 1.0);
}

pub fn metrics_app() -> Router {
    init_metrics();
    
    Router::new()
        .route("/metrics", get(prometheus_metrics))
}

async fn prometheus_metrics() -> String {
    // Get handle to the prometheus recorder
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder()
        .handle()
        .render()
}

// Metric recording helpers
pub fn record_request(endpoint: &str, method: &str) {
    counter!(
        "rs_helper_requests_total",
        "endpoint" => endpoint.to_string(),
        "method" => method.to_string()
    )
    .increment(1);
}

pub fn record_response(endpoint: &str, status: u16, duration: f64) {
    histogram!(
        "rs_helper_request_duration_seconds",
        "endpoint" => endpoint.to_string(),
        "status" => status.to_string()
    )
    .record(duration);
    
    counter!(
        "rs_helper_responses_total",
        "endpoint" => endpoint.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

pub fn record_cache_hit(cache_type: &str) {
    counter!(
        "rs_helper_cache_hits_total",
        "type" => cache_type.to_string()
    )
    .increment(1);
}

pub fn record_cache_miss(cache_type: &str) {
    counter!(
        "rs_helper_cache_misses_total",
        "type" => cache_type.to_string()
    )
    .increment(1);
}

pub fn record_upstream_request(upstream: &str, success: bool, duration: f64) {
    histogram!(
        "rs_helper_upstream_duration_seconds",
        "upstream" => upstream.to_string(),
        "success" => success.to_string()
    )
    .record(duration);
    
    counter!(
        "rs_helper_upstream_requests_total",
        "upstream" => upstream.to_string(),
        "success" => success.to_string()
    )
    .increment(1);
}

pub fn update_cache_size(size: u64) {
    gauge!(
        "rs_helper_cache_entries",
    )
    .set(size as f64);
}

// Request timing middleware helper
pub struct RequestTimer {
    start: Instant,
    endpoint: String,
}

impl RequestTimer {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            endpoint: endpoint.into(),
        }
    }
    
    pub fn record(self, status: u16) {
        let duration = self.start.elapsed().as_secs_f64();
        record_response(&self.endpoint, status, duration);
    }
}
