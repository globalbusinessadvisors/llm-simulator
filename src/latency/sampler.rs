//! Distribution sampling implementations

use rand::prelude::*;
use rand_distr::{Distribution, Exp, LogNormal, Normal, Pareto, Uniform};
use crate::config::LatencyDistribution;

/// Sampler for latency distributions
pub struct DistributionSampler {
    rng: StdRng,
}

impl DistributionSampler {
    /// Create a new sampler with random seed
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a sampler with a fixed seed for reproducibility
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Sample a value from the distribution (in milliseconds)
    pub fn sample(&self, dist: &LatencyDistribution) -> f64 {
        // We need interior mutability for the RNG, so we use a local clone
        let mut rng = self.rng.clone();
        self.sample_with_rng(dist, &mut rng)
    }

    /// Sample a value with an explicit RNG
    pub fn sample_with_rng(&self, dist: &LatencyDistribution, rng: &mut StdRng) -> f64 {
        match dist {
            LatencyDistribution::Fixed { value_ms } => *value_ms,

            LatencyDistribution::Normal { mean_ms, std_dev_ms } => {
                if *std_dev_ms <= 0.0 {
                    return *mean_ms;
                }
                let normal = Normal::new(*mean_ms, *std_dev_ms).unwrap();
                normal.sample(rng).max(0.0)
            }

            LatencyDistribution::LogNormal { mean_ms, std_dev_ms } => {
                if *std_dev_ms <= 0.0 || *mean_ms <= 0.0 {
                    return mean_ms.max(0.0);
                }
                // Convert mean/std to log-normal parameters
                let variance = std_dev_ms.powi(2);
                let mu = (mean_ms.powi(2) / (mean_ms.powi(2) + variance).sqrt()).ln();
                let sigma = (1.0 + variance / mean_ms.powi(2)).ln().sqrt();

                let log_normal = LogNormal::new(mu, sigma).unwrap();
                log_normal.sample(rng)
            }

            LatencyDistribution::Uniform { min_ms, max_ms } => {
                if min_ms >= max_ms {
                    return *min_ms;
                }
                let uniform = Uniform::new(*min_ms, *max_ms);
                uniform.sample(rng)
            }

            LatencyDistribution::Exponential { mean_ms } => {
                if *mean_ms <= 0.0 {
                    return 0.0;
                }
                let lambda = 1.0 / mean_ms;
                let exp = Exp::new(lambda).unwrap();
                exp.sample(rng)
            }

            LatencyDistribution::Pareto { scale_ms, shape } => {
                if *scale_ms <= 0.0 || *shape <= 0.0 {
                    return scale_ms.max(0.0);
                }
                let pareto = Pareto::new(*scale_ms, *shape).unwrap();
                pareto.sample(rng)
            }
        }
    }

    /// Sample multiple values and return statistics
    pub fn sample_n(&self, dist: &LatencyDistribution, n: usize) -> Vec<f64> {
        let mut rng = self.rng.clone();
        (0..n).map(|_| self.sample_with_rng(dist, &mut rng)).collect()
    }
}

impl Default for DistributionSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe sampler that can be shared across async tasks
#[derive(Clone)]
pub struct ThreadSafeSampler {
    seed: Option<u64>,
}

impl ThreadSafeSampler {
    /// Create a new thread-safe sampler
    pub fn new() -> Self {
        Self { seed: None }
    }

    /// Create with a base seed (each sample will derive from this)
    pub fn with_seed(seed: u64) -> Self {
        Self { seed: Some(seed) }
    }

    /// Sample a value from the distribution
    pub fn sample(&self, dist: &LatencyDistribution) -> f64 {
        let sampler = match self.seed {
            Some(seed) => DistributionSampler::with_seed(seed),
            None => DistributionSampler::new(),
        };
        sampler.sample(dist)
    }

    /// Sample TTFT with request-specific seed
    pub fn sample_with_request_seed(&self, dist: &LatencyDistribution, request_id: u64) -> f64 {
        let seed = self.seed.map(|s| s.wrapping_add(request_id));
        let sampler = match seed {
            Some(s) => DistributionSampler::with_seed(s),
            None => DistributionSampler::new(),
        };
        sampler.sample(dist)
    }
}

impl Default for ThreadSafeSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Jitter generator for adding variation
pub struct JitterGenerator {
    base_jitter_ms: f64,
    jitter_factor: f64,
}

impl JitterGenerator {
    pub fn new(base_jitter_ms: f64, jitter_factor: f64) -> Self {
        Self {
            base_jitter_ms,
            jitter_factor,
        }
    }

    /// Add jitter to a base duration
    pub fn add_jitter(&self, base_ms: f64) -> f64 {
        let mut rng = rand::thread_rng();
        let jitter_range = self.base_jitter_ms + (base_ms * self.jitter_factor);
        let jitter = rng.gen_range(-jitter_range..jitter_range);
        (base_ms + jitter).max(0.0)
    }

    /// Generate standalone jitter value
    pub fn generate(&self) -> f64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(0.0..self.base_jitter_ms)
    }
}

impl Default for JitterGenerator {
    fn default() -> Self {
        Self::new(5.0, 0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_distribution() {
        let sampler = DistributionSampler::new();
        let dist = LatencyDistribution::Fixed { value_ms: 100.0 };

        for _ in 0..10 {
            assert_eq!(sampler.sample(&dist), 100.0);
        }
    }

    #[test]
    fn test_normal_distribution() {
        let sampler = DistributionSampler::with_seed(42);
        let dist = LatencyDistribution::Normal {
            mean_ms: 100.0,
            std_dev_ms: 10.0,
        };

        let samples = sampler.sample_n(&dist, 1000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Mean should be close to 100
        assert!((mean - 100.0).abs() < 5.0);
    }

    #[test]
    fn test_uniform_distribution() {
        let sampler = DistributionSampler::with_seed(42);
        let dist = LatencyDistribution::Uniform {
            min_ms: 50.0,
            max_ms: 150.0,
        };

        let samples = sampler.sample_n(&dist, 100);
        for sample in samples {
            assert!(sample >= 50.0);
            assert!(sample < 150.0);
        }
    }

    #[test]
    fn test_exponential_distribution() {
        let sampler = DistributionSampler::with_seed(42);
        let dist = LatencyDistribution::Exponential { mean_ms: 50.0 };

        let samples = sampler.sample_n(&dist, 1000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Mean should be close to 50
        assert!((mean - 50.0).abs() < 10.0);
    }

    #[test]
    fn test_deterministic_sampling() {
        let sampler1 = DistributionSampler::with_seed(42);
        let sampler2 = DistributionSampler::with_seed(42);

        let dist = LatencyDistribution::Normal {
            mean_ms: 100.0,
            std_dev_ms: 20.0,
        };

        let samples1 = sampler1.sample_n(&dist, 10);
        let samples2 = sampler2.sample_n(&dist, 10);

        assert_eq!(samples1, samples2);
    }

    #[test]
    fn test_thread_safe_sampler() {
        let sampler = ThreadSafeSampler::with_seed(42);
        let dist = LatencyDistribution::Fixed { value_ms: 50.0 };

        let result = sampler.sample(&dist);
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_jitter_generator() {
        let jitter = JitterGenerator::new(10.0, 0.1);

        // Should add some variation
        let results: Vec<f64> = (0..10).map(|_| jitter.add_jitter(100.0)).collect();

        // Not all should be exactly 100
        assert!(results.iter().any(|&r| (r - 100.0).abs() > 0.01));
    }

    #[test]
    fn test_log_normal_distribution() {
        let sampler = DistributionSampler::with_seed(42);
        let dist = LatencyDistribution::LogNormal {
            mean_ms: 100.0,
            std_dev_ms: 50.0,
        };

        let samples = sampler.sample_n(&dist, 100);

        // All samples should be positive
        for sample in &samples {
            assert!(*sample > 0.0);
        }
    }

    #[test]
    fn test_pareto_distribution() {
        let sampler = DistributionSampler::with_seed(42);
        let dist = LatencyDistribution::Pareto {
            scale_ms: 10.0,
            shape: 2.0,
        };

        let samples = sampler.sample_n(&dist, 100);

        // All samples should be >= scale
        for sample in &samples {
            assert!(*sample >= 10.0);
        }
    }
}
