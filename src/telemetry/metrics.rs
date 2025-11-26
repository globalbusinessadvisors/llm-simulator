//! Prometheus metrics implementation

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use std::collections::HashMap;

/// Global metrics registry
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, AtomicU64>>,
    gauges: RwLock<HashMap<String, AtomicU64>>,
    histograms: RwLock<HashMap<String, Histogram>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Increment a counter
    pub fn counter_inc(&self, name: &str, value: u64) {
        let counters = self.counters.read();
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write();
            counters.entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Set a gauge value
    pub fn gauge_set(&self, name: &str, value: u64) {
        let mut gauges = self.gauges.write();
        gauges.entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .store(value, Ordering::Relaxed);
    }

    /// Record a histogram observation
    pub fn histogram_observe(&self, name: &str, value: f64) {
        let histograms = self.histograms.read();
        if let Some(hist) = histograms.get(name) {
            hist.observe(value);
        } else {
            drop(histograms);
            let mut histograms = self.histograms.write();
            histograms.entry(name.to_string())
                .or_insert_with(Histogram::new)
                .observe(value);
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (name, counter) in self.counters.read().iter() {
            let value = counter.load(Ordering::Relaxed);
            output.push_str(&format!(
                "# TYPE {} counter\n{} {}\n",
                name, name, value
            ));
        }

        // Export gauges
        for (name, gauge) in self.gauges.read().iter() {
            let value = gauge.load(Ordering::Relaxed);
            output.push_str(&format!(
                "# TYPE {} gauge\n{} {}\n",
                name, name, value
            ));
        }

        // Export histograms
        for (name, hist) in self.histograms.read().iter() {
            let stats = hist.stats();
            output.push_str(&format!("# TYPE {} histogram\n", name));
            output.push_str(&format!("{}_count {}\n", name, stats.count));
            output.push_str(&format!("{}_sum {}\n", name, stats.sum));

            // Buckets
            let buckets = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
            for bucket in buckets {
                let count = hist.count_below(bucket);
                output.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", name, bucket, count));
            }
            output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, stats.count));
        }

        output
    }

    /// Reset all metrics
    pub fn reset(&self) {
        for counter in self.counters.write().values() {
            counter.store(0, Ordering::Relaxed);
        }
        for gauge in self.gauges.write().values() {
            gauge.store(0, Ordering::Relaxed);
        }
        self.histograms.write().clear();
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple histogram implementation
pub struct Histogram {
    values: RwLock<Vec<f64>>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    pub fn new() -> Self {
        Self {
            values: RwLock::new(Vec::with_capacity(10000)),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    pub fn observe(&self, value: f64) {
        let mut values = self.values.write();

        // Reservoir sampling for large datasets
        if values.len() < 10000 {
            values.push(value);
        } else {
            let idx = rand::random::<usize>() % values.len();
            values[idx] = value;
        }

        self.sum.fetch_add(value.to_bits(), Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn count_below(&self, threshold: f64) -> u64 {
        self.values.read()
            .iter()
            .filter(|&&v| v <= threshold)
            .count() as u64
    }

    pub fn stats(&self) -> HistogramStats {
        let values = self.values.read();
        let count = self.count.load(Ordering::Relaxed);

        if values.is_empty() {
            return HistogramStats::default();
        }

        let sum: f64 = values.iter().sum();
        let mean = sum / values.len() as f64;

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentile = |p: f64| -> f64 {
            let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
            sorted[idx.min(sorted.len() - 1)]
        };

        HistogramStats {
            count,
            sum,
            mean,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            p50: percentile(50.0),
            p90: percentile(90.0),
            p95: percentile(95.0),
            p99: percentile(99.0),
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram statistics
#[derive(Debug, Clone, Default)]
pub struct HistogramStats {
    pub count: u64,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Pre-defined metric names
pub mod metric_names {
    // Request metrics
    pub const REQUESTS_TOTAL: &str = "llm_simulator_requests_total";
    pub const REQUESTS_DURATION: &str = "llm_simulator_request_duration_seconds";
    pub const ACTIVE_REQUESTS: &str = "llm_simulator_active_requests";

    // Token metrics
    pub const TOKENS_INPUT: &str = "llm_simulator_tokens_input_total";
    pub const TOKENS_OUTPUT: &str = "llm_simulator_tokens_output_total";

    // Error metrics
    pub const ERRORS_TOTAL: &str = "llm_simulator_errors_total";

    // Streaming metrics
    pub const STREAM_CHUNKS: &str = "llm_simulator_stream_chunks_total";
    pub const TTFT_SECONDS: &str = "llm_simulator_ttft_seconds";
    pub const ITL_SECONDS: &str = "llm_simulator_itl_seconds";

    // Queue metrics (new)
    pub const QUEUE_DEPTH: &str = "llm_simulator_queue_depth";
    pub const QUEUE_CAPACITY: &str = "llm_simulator_queue_capacity";

    // Cost metrics (new)
    pub const COST_DOLLARS: &str = "llm_simulator_cost_dollars_total";

    // Cache metrics (new)
    pub const CACHE_HITS: &str = "llm_simulator_cache_hits_total";
    pub const CACHE_MISSES: &str = "llm_simulator_cache_misses_total";

    // Security metrics (new)
    pub const AUTH_FAILURES: &str = "llm_simulator_auth_failures_total";
    pub const RATE_LIMIT_HITS: &str = "llm_simulator_rate_limit_hits_total";
    pub const CORS_BLOCKED: &str = "llm_simulator_cors_blocked_total";
}

/// Convenience functions for common metrics
pub struct SimulatorMetrics {
    registry: MetricsRegistry,
}

impl SimulatorMetrics {
    pub fn new() -> Self {
        let metrics = Self {
            registry: MetricsRegistry::new(),
        };

        // Initialize gauges with default values
        metrics.set_queue_depth(0);
        metrics.set_queue_capacity(10000);

        metrics
    }

    /// Record a completed request with labels
    pub fn record_request(&self, duration: Duration, model: &str) {
        // Use model-specific metric key for labeling
        let key = format!("{}{{model=\"{}\"}}", metric_names::REQUESTS_TOTAL, model);
        self.registry.counter_inc(&key, 1);

        // Also record total without labels for backwards compatibility
        self.registry.counter_inc(metric_names::REQUESTS_TOTAL, 1);

        let duration_key = format!("{}{{model=\"{}\"}}", metric_names::REQUESTS_DURATION, model);
        self.registry.histogram_observe(&duration_key, duration.as_secs_f64());
        self.registry.histogram_observe(metric_names::REQUESTS_DURATION, duration.as_secs_f64());
    }

    /// Record a request with full labels (provider, model, status)
    pub fn record_request_with_labels(
        &self,
        duration: Duration,
        provider: &str,
        model: &str,
        status: &str,
    ) {
        let key = format!(
            "{}{{provider=\"{}\",model=\"{}\",status=\"{}\"}}",
            metric_names::REQUESTS_TOTAL, provider, model, status
        );
        self.registry.counter_inc(&key, 1);
        self.registry.counter_inc(metric_names::REQUESTS_TOTAL, 1);

        let duration_key = format!(
            "{}{{provider=\"{}\",model=\"{}\"}}",
            metric_names::REQUESTS_DURATION, provider, model
        );
        self.registry.histogram_observe(&duration_key, duration.as_secs_f64());
    }

    /// Record token usage with labels
    pub fn record_tokens(&self, input: u32, output: u32) {
        self.registry.counter_inc(metric_names::TOKENS_INPUT, input as u64);
        self.registry.counter_inc(metric_names::TOKENS_OUTPUT, output as u64);
    }

    /// Record token usage with provider/model labels
    pub fn record_tokens_with_labels(&self, input: u32, output: u32, provider: &str, model: &str) {
        let input_key = format!(
            "{}{{provider=\"{}\",model=\"{}\"}}",
            metric_names::TOKENS_INPUT, provider, model
        );
        let output_key = format!(
            "{}{{provider=\"{}\",model=\"{}\"}}",
            metric_names::TOKENS_OUTPUT, provider, model
        );

        self.registry.counter_inc(&input_key, input as u64);
        self.registry.counter_inc(&output_key, output as u64);
        self.registry.counter_inc(metric_names::TOKENS_INPUT, input as u64);
        self.registry.counter_inc(metric_names::TOKENS_OUTPUT, output as u64);
    }

    /// Record an error with type label
    pub fn record_error(&self, error_type: &str) {
        let key = format!("{}{{error_type=\"{}\"}}", metric_names::ERRORS_TOTAL, error_type);
        self.registry.counter_inc(&key, 1);
        self.registry.counter_inc(metric_names::ERRORS_TOTAL, 1);
    }

    /// Record time to first token
    pub fn record_ttft(&self, ttft: Duration) {
        self.registry.histogram_observe(
            metric_names::TTFT_SECONDS,
            ttft.as_secs_f64(),
        );
    }

    /// Record inter-token latency
    pub fn record_itl(&self, itl: Duration) {
        self.registry.histogram_observe(
            metric_names::ITL_SECONDS,
            itl.as_secs_f64(),
        );
    }

    /// Set the number of active requests
    pub fn set_active_requests(&self, count: u64) {
        self.registry.gauge_set(metric_names::ACTIVE_REQUESTS, count);
    }

    /// Set queue depth (new)
    pub fn set_queue_depth(&self, depth: u64) {
        self.registry.gauge_set(metric_names::QUEUE_DEPTH, depth);
    }

    /// Set queue capacity (new)
    pub fn set_queue_capacity(&self, capacity: u64) {
        self.registry.gauge_set(metric_names::QUEUE_CAPACITY, capacity);
    }

    /// Record cost in dollars (new)
    pub fn record_cost(&self, cost_dollars: f64, provider: &str, model: &str) {
        let key = format!(
            "{}{{provider=\"{}\",model=\"{}\"}}",
            metric_names::COST_DOLLARS, provider, model
        );
        // Store as micro-dollars to avoid floating point issues
        let micro_dollars = (cost_dollars * 1_000_000.0) as u64;
        self.registry.counter_inc(&key, micro_dollars);
    }

    /// Record cache hit (new)
    pub fn record_cache_hit(&self, cache_name: &str) {
        let key = format!("{}{{cache=\"{}\"}}", metric_names::CACHE_HITS, cache_name);
        self.registry.counter_inc(&key, 1);
    }

    /// Record cache miss (new)
    pub fn record_cache_miss(&self, cache_name: &str) {
        let key = format!("{}{{cache=\"{}\"}}", metric_names::CACHE_MISSES, cache_name);
        self.registry.counter_inc(&key, 1);
    }

    /// Record authentication failure (new)
    pub fn record_auth_failure(&self, reason: &str) {
        let key = format!("{}{{reason=\"{}\"}}", metric_names::AUTH_FAILURES, reason);
        self.registry.counter_inc(&key, 1);
    }

    /// Record rate limit hit (new)
    pub fn record_rate_limit_hit(&self, tier: &str) {
        let key = format!("{}{{tier=\"{}\"}}", metric_names::RATE_LIMIT_HITS, tier);
        self.registry.counter_inc(&key, 1);
    }

    /// Record CORS blocked request (new)
    pub fn record_cors_blocked(&self, origin: &str) {
        let key = format!("{}{{origin=\"{}\"}}", metric_names::CORS_BLOCKED, origin);
        self.registry.counter_inc(&key, 1);
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        self.registry.export_prometheus()
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.registry.reset();
    }
}

impl Default for SimulatorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer guard for measuring request duration
pub struct RequestTimer {
    start: Instant,
    metrics: Option<std::sync::Arc<SimulatorMetrics>>,
    model: String,
}

impl RequestTimer {
    pub fn new(metrics: std::sync::Arc<SimulatorMetrics>, model: &str) -> Self {
        Self {
            start: Instant::now(),
            metrics: Some(metrics),
            model: model.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        if let Some(metrics) = &self.metrics {
            metrics.record_request(self.start.elapsed(), &self.model);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        registry.counter_inc("test_counter", 1);
        registry.counter_inc("test_counter", 2);
        registry.gauge_set("test_gauge", 42);
        registry.histogram_observe("test_histogram", 0.5);

        let output = registry.export_prometheus();
        assert!(output.contains("test_counter 3"));
        assert!(output.contains("test_gauge 42"));
        assert!(output.contains("test_histogram"));
    }

    #[test]
    fn test_histogram() {
        let hist = Histogram::new();

        for i in 1..=100 {
            hist.observe(i as f64);
        }

        let stats = hist.stats();
        assert_eq!(stats.count, 100);
        assert!((stats.mean - 50.5).abs() < 1.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 100.0);
    }

    #[test]
    fn test_simulator_metrics() {
        let metrics = SimulatorMetrics::new();

        metrics.record_request(Duration::from_millis(100), "gpt-4");
        metrics.record_tokens(50, 100);
        metrics.record_error("test_error");

        let output = metrics.export();
        assert!(output.contains("llm_simulator_requests_total"));
        assert!(output.contains("llm_simulator_tokens_input_total"));
    }

    #[test]
    fn test_histogram_percentiles() {
        let hist = Histogram::new();

        // Add values 1-100
        for i in 1..=100 {
            hist.observe(i as f64);
        }

        let stats = hist.stats();
        assert!((stats.p50 - 50.0).abs() < 2.0);
        assert!((stats.p90 - 90.0).abs() < 2.0);
        assert!((stats.p99 - 99.0).abs() < 2.0);
    }
}
