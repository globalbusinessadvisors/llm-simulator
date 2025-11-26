//! Property-based tests for configuration validation

use proptest::prelude::*;
use llm_simulator::config::SimulatorConfig;

proptest! {
    /// Test that valid port numbers pass validation
    #[test]
    fn test_valid_port_passes(
        port in 1u16..=65535,
    ) {
        let mut config = SimulatorConfig::default();
        config.server.port = port;

        let result = config.validate();
        prop_assert!(result.is_ok(), "Port {} should be valid", port);
    }

    /// Test that max_concurrent_requests > 0 passes
    #[test]
    fn test_valid_max_concurrent(
        max_concurrent in 1usize..100000,
    ) {
        let mut config = SimulatorConfig::default();
        config.server.max_concurrent_requests = max_concurrent;

        let result = config.validate();
        prop_assert!(result.is_ok());
    }

    /// Test that valid latency multiplier passes
    #[test]
    fn test_valid_latency_multiplier(
        multiplier in 0.1f64..10.0,
    ) {
        let mut config = SimulatorConfig::default();
        config.latency.multiplier = multiplier;

        let result = config.validate();
        prop_assert!(result.is_ok());
    }

    /// Test that valid chaos probability passes
    #[test]
    fn test_valid_chaos_probability(
        probability in 0.0f64..=1.0,
    ) {
        let mut config = SimulatorConfig::default();
        config.chaos.global_probability = probability;

        let result = config.validate();
        prop_assert!(result.is_ok(), "Probability {} should be valid", probability);
    }

    /// Test that host string validation works
    #[test]
    fn test_host_string(
        host in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
    ) {
        let mut config = SimulatorConfig::default();
        config.server.host = host.clone();

        // Just verify it doesn't panic
        let _ = config.validate();
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_zero_port_fails() {
        let mut config = SimulatorConfig::default();
        config.server.port = 0;

        let result = config.validate();
        assert!(result.is_err(), "Port 0 should fail validation");
    }

    #[test]
    fn test_zero_max_concurrent_fails() {
        let mut config = SimulatorConfig::default();
        config.server.max_concurrent_requests = 0;

        let result = config.validate();
        assert!(result.is_err(), "Zero max_concurrent should fail");
    }

    #[test]
    fn test_default_config_valid() {
        let config = SimulatorConfig::default();
        let result = config.validate();
        assert!(result.is_ok(), "Default config should be valid: {:?}", result);
    }

    #[test]
    fn test_config_with_models() {
        let config = SimulatorConfig::default();

        // Should have default models
        assert!(config.models.len() > 0);
        assert!(config.models.contains_key("gpt-4"));
        assert!(config.models.contains_key("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_get_model() {
        let config = SimulatorConfig::default();

        let gpt4 = config.get_model("gpt-4");
        assert!(gpt4.is_some());

        let nonexistent = config.get_model("nonexistent-model");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_chaos_config_validation() {
        let mut config = SimulatorConfig::default();

        // Invalid probability
        config.chaos.global_probability = 1.5;
        let result = config.chaos.validate();
        assert!(result.is_err(), "Probability > 1 should fail");

        // Negative probability
        config.chaos.global_probability = -0.1;
        let result = config.chaos.validate();
        assert!(result.is_err(), "Negative probability should fail");

        // Valid probability
        config.chaos.global_probability = 0.5;
        let result = config.chaos.validate();
        assert!(result.is_ok());
    }
}
