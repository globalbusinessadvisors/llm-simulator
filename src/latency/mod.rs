//! Latency simulation module
//!
//! Provides realistic latency simulation using statistical distributions
//! for Time-To-First-Token (TTFT) and Inter-Token-Latency (ITL).

mod sampler;

pub use sampler::*;

use std::time::Duration;
use crate::config::{LatencyConfig, LatencyProfile};

/// Latency simulator that generates realistic timing
#[derive(Debug, Clone)]
pub struct LatencySimulator {
    config: LatencyConfig,
    rng_seed: Option<u64>,
}

impl LatencySimulator {
    /// Create a new latency simulator with the given configuration
    pub fn new(config: LatencyConfig) -> Self {
        Self {
            config,
            rng_seed: None,
        }
    }

    /// Create a latency simulator with a fixed seed for deterministic behavior
    pub fn with_seed(config: LatencyConfig, seed: u64) -> Self {
        Self {
            config,
            rng_seed: Some(seed),
        }
    }

    /// Check if latency simulation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configured multiplier
    pub fn multiplier(&self) -> f64 {
        self.config.multiplier
    }

    /// Generate a TTFT (Time to First Token) duration
    pub fn sample_ttft(&self, profile_name: Option<&str>) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        let profile = profile_name
            .and_then(|name| self.config.get_profile(name))
            .unwrap_or_else(|| self.config.default_profile());

        let sampler = self.create_sampler();
        let base_ms = sampler.sample(&profile.ttft);
        let adjusted_ms = base_ms * self.config.multiplier;

        Duration::from_micros((adjusted_ms * 1000.0).max(0.0) as u64)
    }

    /// Generate an ITL (Inter-Token Latency) duration
    pub fn sample_itl(&self, profile_name: Option<&str>) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        let profile = profile_name
            .and_then(|name| self.config.get_profile(name))
            .unwrap_or_else(|| self.config.default_profile());

        let sampler = self.create_sampler();
        let base_ms = sampler.sample(&profile.itl);
        let adjusted_ms = base_ms * self.config.multiplier;

        Duration::from_micros((adjusted_ms * 1000.0).max(0.0) as u64)
    }

    /// Get the fixed overhead for a profile
    pub fn overhead(&self, profile_name: Option<&str>) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        let profile = profile_name
            .and_then(|name| self.config.get_profile(name))
            .unwrap_or_else(|| self.config.default_profile());

        Duration::from_micros((profile.overhead.as_micros() as f64 * self.config.multiplier) as u64)
    }

    /// Generate a complete latency schedule for a streaming response
    pub fn generate_schedule(&self, token_count: usize, profile_name: Option<&str>) -> LatencySchedule {
        let ttft = self.sample_ttft(profile_name);
        let overhead = self.overhead(profile_name);

        let mut token_delays = Vec::with_capacity(token_count);
        for _ in 0..token_count {
            token_delays.push(self.sample_itl(profile_name));
        }

        LatencySchedule {
            ttft,
            overhead,
            token_delays,
        }
    }

    /// Create a distribution sampler
    fn create_sampler(&self) -> DistributionSampler {
        match self.rng_seed {
            Some(seed) => DistributionSampler::with_seed(seed),
            None => DistributionSampler::new(),
        }
    }

    /// Get available profile names
    pub fn profile_names(&self) -> Vec<&str> {
        self.config.profiles.keys().map(|s| s.as_str()).collect()
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&LatencyProfile> {
        self.config.get_profile(name)
    }
}

impl Default for LatencySimulator {
    fn default() -> Self {
        Self::new(LatencyConfig::default())
    }
}

/// A schedule of latencies for a complete response
#[derive(Debug, Clone)]
pub struct LatencySchedule {
    /// Time to first token
    pub ttft: Duration,
    /// Fixed overhead
    pub overhead: Duration,
    /// Delay for each token
    pub token_delays: Vec<Duration>,
}

impl LatencySchedule {
    /// Get total expected duration
    pub fn total_duration(&self) -> Duration {
        let token_total: Duration = self.token_delays.iter().sum();
        self.ttft + self.overhead + token_total
    }

    /// Get delay for a specific token index
    pub fn delay_for_token(&self, index: usize) -> Duration {
        if index == 0 {
            self.ttft + self.overhead
        } else {
            self.token_delays.get(index).copied().unwrap_or(Duration::ZERO)
        }
    }

    /// Create an instant schedule (no delays)
    pub fn instant(token_count: usize) -> Self {
        Self {
            ttft: Duration::ZERO,
            overhead: Duration::ZERO,
            token_delays: vec![Duration::ZERO; token_count],
        }
    }
}

/// Statistics from latency sampling
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub samples: u64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub std_dev_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl LatencyStats {
    /// Compute statistics from a set of samples
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentile = |p: f64| -> f64 {
            let idx = (p * (sorted.len() - 1) as f64) as usize;
            sorted[idx]
        };

        Self {
            samples: samples.len() as u64,
            min_ms: sorted[0],
            max_ms: sorted[sorted.len() - 1],
            mean_ms: mean,
            std_dev_ms: std_dev,
            p50_ms: percentile(0.50),
            p95_ms: percentile(0.95),
            p99_ms: percentile(0.99),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let sim = LatencySimulator::default();
        assert!(sim.is_enabled());
    }

    #[test]
    fn test_disabled_simulator() {
        let mut config = LatencyConfig::default();
        config.enabled = false;
        let sim = LatencySimulator::new(config);

        assert_eq!(sim.sample_ttft(None), Duration::ZERO);
        assert_eq!(sim.sample_itl(None), Duration::ZERO);
    }

    #[test]
    fn test_deterministic_with_seed() {
        let config = LatencyConfig::default();
        let sim1 = LatencySimulator::with_seed(config.clone(), 42);
        let sim2 = LatencySimulator::with_seed(config, 42);

        // With same seed, should produce same results
        let ttft1 = sim1.sample_ttft(Some("standard"));
        let ttft2 = sim2.sample_ttft(Some("standard"));
        assert_eq!(ttft1, ttft2);
    }

    #[test]
    fn test_latency_schedule() {
        let sim = LatencySimulator::default();
        let schedule = sim.generate_schedule(10, Some("fast"));

        assert_eq!(schedule.token_delays.len(), 10);
        assert!(schedule.total_duration() > Duration::ZERO);
    }

    #[test]
    fn test_instant_schedule() {
        let schedule = LatencySchedule::instant(5);
        assert_eq!(schedule.total_duration(), Duration::ZERO);
        assert_eq!(schedule.token_delays.len(), 5);
    }

    #[test]
    fn test_multiplier() {
        let mut config = LatencyConfig::default();
        config.multiplier = 2.0;
        let sim_2x = LatencySimulator::with_seed(config.clone(), 42);

        config.multiplier = 1.0;
        let sim_1x = LatencySimulator::with_seed(config, 42);

        // 2x multiplier should approximately double the latency
        let ttft_1x = sim_1x.sample_ttft(Some("instant"));
        let ttft_2x = sim_2x.sample_ttft(Some("instant"));

        // For instant profile, both should be near zero
        assert!(ttft_1x < Duration::from_millis(1));
        assert!(ttft_2x < Duration::from_millis(1));
    }

    #[test]
    fn test_latency_stats() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = LatencyStats::from_samples(&samples);

        assert_eq!(stats.samples, 5);
        assert_eq!(stats.min_ms, 10.0);
        assert_eq!(stats.max_ms, 50.0);
        assert_eq!(stats.mean_ms, 30.0);
    }
}
