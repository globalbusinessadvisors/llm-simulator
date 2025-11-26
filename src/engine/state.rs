//! Engine state and statistics tracking

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Thread-safe engine state tracking
pub struct EngineState {
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_input_tokens: AtomicU64,
    total_output_tokens: AtomicU64,
    latencies: RwLock<LatencyTracker>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_input_tokens: AtomicU64::new(0),
            total_output_tokens: AtomicU64::new(0),
            latencies: RwLock::new(LatencyTracker::new()),
        }
    }

    /// Increment request counter
    pub fn increment_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error counter
    pub fn increment_errors(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Add token counts
    pub fn add_tokens(&self, input: u64, output: u64) {
        self.total_input_tokens.fetch_add(input, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    /// Record a latency measurement
    pub fn record_latency(&self, latency: Duration) {
        self.latencies.write().record(latency);
    }

    /// Get current statistics
    pub fn stats(&self) -> EngineStats {
        let latency_stats = self.latencies.read().stats();

        EngineStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            total_input_tokens: self.total_input_tokens.load(Ordering::Relaxed),
            total_output_tokens: self.total_output_tokens.load(Ordering::Relaxed),
            latency: latency_stats,
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_errors.store(0, Ordering::Relaxed);
        self.total_input_tokens.store(0, Ordering::Relaxed);
        self.total_output_tokens.store(0, Ordering::Relaxed);
        *self.latencies.write() = LatencyTracker::new();
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Engine statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub latency: LatencyStats,
}

impl EngineStats {
    /// Calculate error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_requests as f64
        }
    }

    /// Calculate tokens per request
    pub fn tokens_per_request(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_input_tokens + self.total_output_tokens) as f64 / self.total_requests as f64
        }
    }
}

impl Default for EngineStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            latency: LatencyStats::default(),
        }
    }
}

/// Tracks latency measurements with reservoir sampling
struct LatencyTracker {
    samples: Vec<Duration>,
    count: u64,
    max_samples: usize,
    sum: Duration,
    min: Option<Duration>,
    max: Option<Duration>,
}

impl LatencyTracker {
    fn new() -> Self {
        Self::with_capacity(10_000)
    }

    fn with_capacity(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            count: 0,
            max_samples,
            sum: Duration::ZERO,
            min: None,
            max: None,
        }
    }

    fn record(&mut self, latency: Duration) {
        self.count += 1;
        self.sum += latency;

        // Update min/max
        self.min = Some(self.min.map_or(latency, |m| m.min(latency)));
        self.max = Some(self.max.map_or(latency, |m| m.max(latency)));

        // Reservoir sampling for percentiles
        if self.samples.len() < self.max_samples {
            self.samples.push(latency);
        } else {
            // Reservoir sampling: replace with probability max_samples/count
            let idx = rand::random::<usize>() % self.count as usize;
            if idx < self.max_samples {
                self.samples[idx] = latency;
            }
        }
    }

    fn stats(&self) -> LatencyStats {
        if self.count == 0 {
            return LatencyStats::default();
        }

        let mean = self.sum.as_secs_f64() * 1000.0 / self.count as f64;

        // Calculate percentiles from samples
        let mut sorted: Vec<f64> = self.samples.iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentile = |p: f64| -> f64 {
            if sorted.is_empty() {
                return 0.0;
            }
            let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
            sorted[idx.min(sorted.len() - 1)]
        };

        LatencyStats {
            count: self.count,
            mean_ms: mean,
            min_ms: self.min.map_or(0.0, |d| d.as_secs_f64() * 1000.0),
            max_ms: self.max.map_or(0.0, |d| d.as_secs_f64() * 1000.0),
            p50_ms: percentile(50.0),
            p90_ms: percentile(90.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
        }
    }
}

/// Latency statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: u64,
    pub mean_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

/// Request-level metrics
#[derive(Debug, Clone, Serialize)]
pub struct RequestMetrics {
    pub request_id: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub ttft_ms: f64,
    pub total_latency_ms: f64,
    pub streaming: bool,
}

/// Session state for multi-turn conversations
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub messages: Vec<crate::types::Message>,
    pub total_tokens: usize,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

impl SessionState {
    pub fn new(session_id: String) -> Self {
        let now = std::time::Instant::now();
        Self {
            session_id,
            messages: Vec::new(),
            total_tokens: 0,
            created_at: now,
            last_activity: now,
        }
    }

    pub fn add_message(&mut self, message: crate::types::Message) {
        self.total_tokens += message.estimate_tokens();
        self.messages.push(message);
        self.last_activity = std::time::Instant::now();
    }

    pub fn is_expired(&self, max_idle: Duration) -> bool {
        self.last_activity.elapsed() > max_idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_state() {
        let state = EngineState::new();

        state.increment_requests();
        state.increment_requests();
        state.increment_errors();
        state.add_tokens(100, 50);
        state.record_latency(Duration::from_millis(100));

        let stats = state.stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);
    }

    #[test]
    fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new();

        tracker.record(Duration::from_millis(100));
        tracker.record(Duration::from_millis(200));
        tracker.record(Duration::from_millis(300));

        let stats = tracker.stats();
        assert_eq!(stats.count, 3);
        assert!((stats.mean_ms - 200.0).abs() < 1.0);
        assert!((stats.min_ms - 100.0).abs() < 1.0);
        assert!((stats.max_ms - 300.0).abs() < 1.0);
    }

    #[test]
    fn test_error_rate() {
        let stats = EngineStats {
            total_requests: 100,
            total_errors: 10,
            ..Default::default()
        };

        assert!((stats.error_rate() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let state = EngineState::new();
        state.increment_requests();
        state.add_tokens(100, 50);

        state.reset();

        let stats = state.stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_input_tokens, 0);
    }

    #[test]
    fn test_session_state() {
        let mut session = SessionState::new("test-session".to_string());

        session.add_message(crate::types::Message::user("Hello"));
        session.add_message(crate::types::Message::assistant("Hi there!"));

        assert_eq!(session.messages.len(), 2);
        assert!(session.total_tokens > 0);
        assert!(!session.is_expired(Duration::from_secs(60)));
    }
}
