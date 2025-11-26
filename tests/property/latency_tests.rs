//! Property-based tests for latency distribution

use proptest::prelude::*;
use llm_simulator::config::LatencyDistribution;
use llm_simulator::latency::DistributionSampler;

proptest! {
    /// Test that normal distribution produces positive values
    #[test]
    fn test_normal_distribution_positive(
        mean in 10.0f64..1000.0,
        std_dev in 1.0f64..100.0,
    ) {
        let dist = LatencyDistribution::Normal { mean_ms: mean, std_dev_ms: std_dev };
        let sampler = DistributionSampler::new();

        // Sample multiple times
        for _ in 0..100 {
            let sample = sampler.sample(&dist);
            // Values should be positive (clamped at 0)
            prop_assert!(sample >= 0.0, "Sample {} should be >= 0", sample);
        }
    }

    /// Test that uniform distribution stays within bounds
    #[test]
    fn test_uniform_distribution_bounds(
        min in 1.0f64..100.0,
        range in 1.0f64..100.0,
    ) {
        let max = min + range;
        let dist = LatencyDistribution::Uniform { min_ms: min, max_ms: max };
        let sampler = DistributionSampler::new();

        for _ in 0..100 {
            let sample = sampler.sample(&dist);
            prop_assert!(
                sample >= min && sample <= max,
                "Sample {} should be in [{}, {}]",
                sample, min, max
            );
        }
    }

    /// Test that exponential distribution produces positive values
    #[test]
    fn test_exponential_distribution_positive(
        mean in 1.0f64..1000.0,
    ) {
        let dist = LatencyDistribution::Exponential { mean_ms: mean };
        let sampler = DistributionSampler::new();

        for _ in 0..100 {
            let sample = sampler.sample(&dist);
            prop_assert!(sample >= 0.0, "Exponential sample {} should be >= 0", sample);
        }
    }

    /// Test that fixed distribution returns the fixed value
    #[test]
    fn test_fixed_distribution(
        value in 0.0f64..1000.0,
    ) {
        let dist = LatencyDistribution::Fixed { value_ms: value };
        let sampler = DistributionSampler::new();

        for _ in 0..10 {
            let sample = sampler.sample(&dist);
            prop_assert!(
                (sample - value).abs() < 0.001,
                "Fixed sample {} should equal {}",
                sample, value
            );
        }
    }

    /// Test that seeded samplers produce deterministic results
    #[test]
    fn test_seeded_sampler_deterministic(
        seed in 0u64..u64::MAX,
        mean in 10.0f64..1000.0,
    ) {
        let dist = LatencyDistribution::Normal { mean_ms: mean, std_dev_ms: 10.0 };

        let sampler1 = DistributionSampler::with_seed(seed);
        let sampler2 = DistributionSampler::with_seed(seed);

        // Same seed should produce same sequence
        for _ in 0..10 {
            let s1 = sampler1.sample(&dist);
            let s2 = sampler2.sample(&dist);
            prop_assert!(
                (s1 - s2).abs() < 0.0001,
                "Seeded samples should match: {} vs {}",
                s1, s2
            );
        }
    }

    /// Test that normal distribution mean is approximately correct
    #[test]
    fn test_normal_distribution_mean_approximation(
        target_mean in 50.0f64..500.0,
    ) {
        let dist = LatencyDistribution::Normal {
            mean_ms: target_mean,
            std_dev_ms: target_mean * 0.1, // 10% std dev
        };
        let sampler = DistributionSampler::with_seed(42);

        // Collect samples
        let samples: Vec<f64> = (0..1000).map(|_| sampler.sample(&dist)).collect();
        let actual_mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Mean should be within 20% of target (statistical tolerance)
        let tolerance = target_mean * 0.20;
        prop_assert!(
            (actual_mean - target_mean).abs() < tolerance,
            "Mean {} should be within {} of target {}",
            actual_mean, tolerance, target_mean
        );
    }

    /// Test that log-normal distribution produces positive values
    #[test]
    fn test_lognormal_distribution_positive(
        mean in 10.0f64..500.0,
        std_dev in 1.0f64..50.0,
    ) {
        let dist = LatencyDistribution::LogNormal { mean_ms: mean, std_dev_ms: std_dev };
        let sampler = DistributionSampler::new();

        for _ in 0..100 {
            let sample = sampler.sample(&dist);
            prop_assert!(sample > 0.0, "LogNormal sample {} should be > 0", sample);
        }
    }

    /// Test that pareto distribution values are >= scale
    #[test]
    fn test_pareto_distribution_bounds(
        scale in 1.0f64..100.0,
        shape in 1.0f64..10.0,
    ) {
        let dist = LatencyDistribution::Pareto { scale_ms: scale, shape };
        let sampler = DistributionSampler::new();

        for _ in 0..100 {
            let sample = sampler.sample(&dist);
            prop_assert!(
                sample >= scale,
                "Pareto sample {} should be >= scale {}",
                sample, scale
            );
        }
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_distribution_constructors() {
        // Test helper constructors
        let fixed = LatencyDistribution::fixed(100.0);
        let normal = LatencyDistribution::normal(50.0, 10.0);
        let uniform = LatencyDistribution::uniform(10.0, 100.0);
        let exponential = LatencyDistribution::exponential(50.0);

        let sampler = DistributionSampler::with_seed(42);

        assert_eq!(sampler.sample(&fixed), 100.0);
        assert!(sampler.sample(&normal) > 0.0);
        assert!(sampler.sample(&uniform) >= 10.0);
        assert!(sampler.sample(&exponential) >= 0.0);
    }

    #[test]
    fn test_sample_n() {
        let dist = LatencyDistribution::Normal { mean_ms: 100.0, std_dev_ms: 10.0 };
        let sampler = DistributionSampler::with_seed(42);

        let samples = sampler.sample_n(&dist, 100);
        assert_eq!(samples.len(), 100);

        // All should be positive
        assert!(samples.iter().all(|&s| s >= 0.0));
    }

    #[test]
    fn test_default_sampler() {
        let sampler = DistributionSampler::default();
        let dist = LatencyDistribution::Fixed { value_ms: 42.0 };
        assert_eq!(sampler.sample(&dist), 42.0);
    }
}
