//! Chaos engineering configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::error::{InjectedErrorType, SimulationError, SimulatorResult};

/// Chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChaosConfig {
    /// Enable chaos engineering
    pub enabled: bool,
    /// Global probability multiplier (0.0-1.0)
    pub global_probability: f64,
    /// Error injection rules
    pub errors: Vec<ErrorInjectionRule>,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            global_probability: 1.0,
            errors: vec![],
            circuit_breaker: CircuitBreakerConfig::default(),
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

impl ChaosConfig {
    pub fn validate(&self) -> SimulatorResult<()> {
        if !(0.0..=1.0).contains(&self.global_probability) {
            return Err(SimulationError::Validation {
                message: "global_probability must be between 0.0 and 1.0".to_string(),
                param: Some("chaos.global_probability".to_string()),
            });
        }
        for (i, rule) in self.errors.iter().enumerate() {
            rule.validate().map_err(|e| SimulationError::Validation {
                message: format!("Invalid error rule {}: {}", i, e),
                param: Some(format!("chaos.errors[{}]", i)),
            })?;
        }
        self.circuit_breaker.validate()?;
        self.rate_limiting.validate()?;
        Ok(())
    }

    /// Check if chaos is active
    pub fn is_active(&self) -> bool {
        self.enabled && self.global_probability > 0.0
    }
}

/// Rule for injecting errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInjectionRule {
    /// Name of the rule
    pub name: String,
    /// Type of error to inject
    pub error_type: InjectedErrorType,
    /// Probability of injection (0.0-1.0)
    pub probability: f64,
    /// Models this rule applies to (empty = all)
    #[serde(default)]
    pub models: Vec<String>,
    /// Endpoints this rule applies to (empty = all)
    #[serde(default)]
    pub endpoints: Vec<String>,
    /// Custom error message
    pub message: Option<String>,
    /// HTTP status code override
    pub status_code: Option<u16>,
    /// Additional delay to add (ms)
    pub delay_ms: Option<u64>,
    /// Whether the rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl ErrorInjectionRule {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Rule name cannot be empty".to_string());
        }
        if !(0.0..=1.0).contains(&self.probability) {
            return Err("Probability must be between 0.0 and 1.0".to_string());
        }
        if let Some(code) = self.status_code {
            if !(100..=599).contains(&code) {
                return Err("Status code must be between 100 and 599".to_string());
            }
        }
        Ok(())
    }

    /// Check if this rule applies to the given model
    pub fn applies_to_model(&self, model: &str) -> bool {
        self.models.is_empty() || self.models.iter().any(|m| m == model || model.starts_with(m))
    }

    /// Check if this rule applies to the given endpoint
    pub fn applies_to_endpoint(&self, endpoint: &str) -> bool {
        self.endpoints.is_empty() || self.endpoints.iter().any(|e| endpoint.contains(e))
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Number of failures before opening
    pub failure_threshold: u32,
    /// Time window for counting failures (seconds)
    pub failure_window_secs: u64,
    /// Time to wait before half-open state (seconds)
    pub recovery_timeout_secs: u64,
    /// Number of successes needed to close from half-open
    pub success_threshold: u32,
    /// Per-model circuit breakers
    pub per_model: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: 5,
            failure_window_secs: 60,
            recovery_timeout_secs: 30,
            success_threshold: 3,
            per_model: true,
        }
    }
}

impl CircuitBreakerConfig {
    pub fn validate(&self) -> SimulatorResult<()> {
        if self.failure_threshold == 0 {
            return Err(SimulationError::Validation {
                message: "failure_threshold must be greater than 0".to_string(),
                param: Some("chaos.circuit_breaker.failure_threshold".to_string()),
            });
        }
        Ok(())
    }

    /// Get the failure window duration
    pub fn failure_window(&self) -> Duration {
        Duration::from_secs(self.failure_window_secs)
    }

    /// Get the recovery timeout duration
    pub fn recovery_timeout(&self) -> Duration {
        Duration::from_secs(self.recovery_timeout_secs)
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Default requests per minute
    pub requests_per_minute: u32,
    /// Default tokens per minute
    pub tokens_per_minute: u32,
    /// Per-model rate limits
    pub model_limits: HashMap<String, ModelRateLimit>,
    /// Burst allowance (multiplier)
    pub burst_multiplier: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut model_limits = HashMap::new();

        // GPT-4 class models (lower limits)
        model_limits.insert("gpt-4".to_string(), ModelRateLimit {
            requests_per_minute: 500,
            tokens_per_minute: 40_000,
        });

        // GPT-3.5 class models (higher limits)
        model_limits.insert("gpt-3.5".to_string(), ModelRateLimit {
            requests_per_minute: 3500,
            tokens_per_minute: 90_000,
        });

        // Claude models
        model_limits.insert("claude".to_string(), ModelRateLimit {
            requests_per_minute: 1000,
            tokens_per_minute: 100_000,
        });

        Self {
            enabled: false,
            requests_per_minute: 1000,
            tokens_per_minute: 100_000,
            model_limits,
            burst_multiplier: 1.5,
        }
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> SimulatorResult<()> {
        if self.burst_multiplier < 1.0 {
            return Err(SimulationError::Validation {
                message: "burst_multiplier must be >= 1.0".to_string(),
                param: Some("chaos.rate_limiting.burst_multiplier".to_string()),
            });
        }
        Ok(())
    }

    /// Get rate limit for a specific model
    pub fn get_limit(&self, model: &str) -> ModelRateLimit {
        // Check for exact match first
        if let Some(limit) = self.model_limits.get(model) {
            return limit.clone();
        }

        // Check for prefix match
        for (prefix, limit) in &self.model_limits {
            if model.starts_with(prefix) {
                return limit.clone();
            }
        }

        // Return default
        ModelRateLimit {
            requests_per_minute: self.requests_per_minute,
            tokens_per_minute: self.tokens_per_minute,
        }
    }
}

/// Per-model rate limit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
}

impl ModelRateLimit {
    /// Calculate retry-after duration based on tokens used
    pub fn retry_after(&self, tokens_used: u32) -> Duration {
        if self.tokens_per_minute == 0 {
            return Duration::from_secs(60);
        }
        let seconds = (tokens_used as f64 / self.tokens_per_minute as f64) * 60.0;
        Duration::from_secs_f64(seconds.max(1.0).min(300.0))
    }
}

/// Predefined chaos scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChaosScenario {
    /// No chaos - normal operation
    None,
    /// Occasional timeouts
    IntermittentTimeouts,
    /// High rate limit pressure
    RateLimitStress,
    /// Degraded performance
    HighLatency,
    /// Service degradation
    PartialOutage,
    /// Complete service failure
    FullOutage,
    /// Custom configuration
    Custom,
}

impl ChaosScenario {
    /// Apply a predefined scenario to the chaos config
    pub fn apply(&self, config: &mut ChaosConfig) {
        match self {
            Self::None => {
                config.enabled = false;
            }
            Self::IntermittentTimeouts => {
                config.enabled = true;
                config.errors = vec![
                    ErrorInjectionRule {
                        name: "random_timeout".to_string(),
                        error_type: InjectedErrorType::Timeout,
                        probability: 0.05,
                        models: vec![],
                        endpoints: vec![],
                        message: Some("Request timed out".to_string()),
                        status_code: Some(504),
                        delay_ms: Some(30000),
                        enabled: true,
                    },
                ];
            }
            Self::RateLimitStress => {
                config.enabled = true;
                config.rate_limiting.enabled = true;
                config.rate_limiting.requests_per_minute = 10;
                config.rate_limiting.tokens_per_minute = 1000;
                config.errors = vec![
                    ErrorInjectionRule {
                        name: "rate_limit".to_string(),
                        error_type: InjectedErrorType::RateLimit,
                        probability: 0.3,
                        models: vec![],
                        endpoints: vec![],
                        message: Some("Rate limit exceeded".to_string()),
                        status_code: Some(429),
                        delay_ms: None,
                        enabled: true,
                    },
                ];
            }
            Self::HighLatency => {
                config.enabled = true;
                config.errors = vec![
                    ErrorInjectionRule {
                        name: "high_latency".to_string(),
                        error_type: InjectedErrorType::Timeout,
                        probability: 0.0, // No actual errors, just delay
                        models: vec![],
                        endpoints: vec![],
                        message: None,
                        status_code: None,
                        delay_ms: Some(5000),
                        enabled: true,
                    },
                ];
            }
            Self::PartialOutage => {
                config.enabled = true;
                config.circuit_breaker.enabled = true;
                config.errors = vec![
                    ErrorInjectionRule {
                        name: "server_error".to_string(),
                        error_type: InjectedErrorType::ServerError,
                        probability: 0.25,
                        models: vec![],
                        endpoints: vec![],
                        message: Some("Internal server error".to_string()),
                        status_code: Some(500),
                        delay_ms: None,
                        enabled: true,
                    },
                    ErrorInjectionRule {
                        name: "service_unavailable".to_string(),
                        error_type: InjectedErrorType::ServiceUnavailable,
                        probability: 0.1,
                        models: vec![],
                        endpoints: vec![],
                        message: Some("Service temporarily unavailable".to_string()),
                        status_code: Some(503),
                        delay_ms: None,
                        enabled: true,
                    },
                ];
            }
            Self::FullOutage => {
                config.enabled = true;
                config.errors = vec![
                    ErrorInjectionRule {
                        name: "full_outage".to_string(),
                        error_type: InjectedErrorType::ServiceUnavailable,
                        probability: 1.0,
                        models: vec![],
                        endpoints: vec![],
                        message: Some("Service is currently unavailable".to_string()),
                        status_code: Some(503),
                        delay_ms: None,
                        enabled: true,
                    },
                ];
            }
            Self::Custom => {
                // No changes - use existing config
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChaosConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.global_probability, 1.0);
    }

    #[test]
    fn test_config_validation() {
        let config = ChaosConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_error_rule_validation() {
        let valid = ErrorInjectionRule {
            name: "test".to_string(),
            error_type: InjectedErrorType::Timeout,
            probability: 0.5,
            models: vec![],
            endpoints: vec![],
            message: None,
            status_code: None,
            delay_ms: None,
            enabled: true,
        };
        assert!(valid.validate().is_ok());

        let invalid = ErrorInjectionRule {
            probability: 1.5, // Invalid
            ..valid.clone()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_rule_applies_to_model() {
        let rule = ErrorInjectionRule {
            name: "test".to_string(),
            error_type: InjectedErrorType::Timeout,
            probability: 0.5,
            models: vec!["gpt-4".to_string()],
            endpoints: vec![],
            message: None,
            status_code: None,
            delay_ms: None,
            enabled: true,
        };

        assert!(rule.applies_to_model("gpt-4"));
        assert!(rule.applies_to_model("gpt-4-turbo"));
        assert!(!rule.applies_to_model("gpt-3.5-turbo"));
    }

    #[test]
    fn test_chaos_scenario() {
        let mut config = ChaosConfig::default();
        ChaosScenario::IntermittentTimeouts.apply(&mut config);
        assert!(config.enabled);
        assert!(!config.errors.is_empty());
    }

    #[test]
    fn test_rate_limit_lookup() {
        let config = RateLimitConfig::default();

        // Exact match
        let gpt4 = config.get_limit("gpt-4");
        assert_eq!(gpt4.requests_per_minute, 500);

        // Prefix match
        let gpt4_turbo = config.get_limit("gpt-4-turbo");
        assert_eq!(gpt4_turbo.requests_per_minute, 500);

        // Default fallback
        let unknown = config.get_limit("unknown-model");
        assert_eq!(unknown.requests_per_minute, config.requests_per_minute);
    }
}
