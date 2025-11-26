//! Chaos engineering implementation

use rand::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use parking_lot::RwLock;
use std::collections::HashMap;

use crate::config::{ChaosConfig, ErrorInjectionRule, CircuitBreakerConfig};
use crate::error::{SimulationError, InjectedErrorType};

/// Chaos engineering engine for error injection and circuit breaking
pub struct ChaosEngine {
    config: ChaosConfig,
    circuit_breakers: RwLock<HashMap<String, CircuitBreaker>>,
    request_counter: AtomicU64,
}

impl ChaosEngine {
    /// Create a new chaos engine
    pub fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            circuit_breakers: RwLock::new(HashMap::new()),
            request_counter: AtomicU64::new(0),
        }
    }

    /// Check if chaos is active
    pub fn is_active(&self) -> bool {
        self.config.is_active()
    }

    /// Maybe inject an error based on configuration
    pub fn maybe_inject_error(&self, model: &str, endpoint: &str) -> Option<SimulationError> {
        if !self.config.is_active() {
            return None;
        }

        let _request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);

        // Check circuit breaker first
        if self.config.circuit_breaker.enabled {
            if let Some(error) = self.check_circuit_breaker(model) {
                return Some(error);
            }
        }

        // Check rate limiting
        if self.config.rate_limiting.enabled {
            if let Some(error) = self.check_rate_limit(model) {
                return Some(error);
            }
        }

        // Check error injection rules
        let mut rng = rand::thread_rng();

        for rule in &self.config.errors {
            if !rule.enabled {
                continue;
            }

            if !rule.applies_to_model(model) {
                continue;
            }

            if !rule.applies_to_endpoint(endpoint) {
                continue;
            }

            // Apply global probability multiplier
            let effective_prob = rule.probability * self.config.global_probability;

            if rng.gen::<f64>() < effective_prob {
                return Some(self.create_error(rule));
            }
        }

        None
    }

    /// Create an error from a rule
    fn create_error(&self, rule: &ErrorInjectionRule) -> SimulationError {
        let message = rule.message.clone()
            .unwrap_or_else(|| format!("Injected {} error", rule.error_type));

        let status_code = rule.status_code.unwrap_or_else(|| {
            match rule.error_type {
                InjectedErrorType::RateLimit => 429,
                InjectedErrorType::Timeout => 504,
                InjectedErrorType::ServerError => 500,
                InjectedErrorType::BadGateway => 502,
                InjectedErrorType::ServiceUnavailable => 503,
                InjectedErrorType::AuthenticationError => 401,
                InjectedErrorType::InvalidRequest => 400,
                InjectedErrorType::ContextLengthExceeded => 400,
            }
        });

        SimulationError::Injected {
            error_type: rule.error_type,
            message,
            status_code,
        }
    }

    /// Check circuit breaker state
    fn check_circuit_breaker(&self, model: &str) -> Option<SimulationError> {
        let key = if self.config.circuit_breaker.per_model {
            model.to_string()
        } else {
            "global".to_string()
        };

        let mut breakers = self.circuit_breakers.write();
        let breaker = breakers.entry(key).or_insert_with(|| {
            CircuitBreaker::new(self.config.circuit_breaker.clone())
        });

        if breaker.is_open() {
            Some(SimulationError::ServiceUnavailable(
                "Circuit breaker is open".to_string()
            ))
        } else {
            None
        }
    }

    /// Record a failure for circuit breaker
    pub fn record_failure(&self, model: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        let key = if self.config.circuit_breaker.per_model {
            model.to_string()
        } else {
            "global".to_string()
        };

        let mut breakers = self.circuit_breakers.write();
        if let Some(breaker) = breakers.get_mut(&key) {
            breaker.record_failure();
        }
    }

    /// Record a success for circuit breaker
    pub fn record_success(&self, model: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        let key = if self.config.circuit_breaker.per_model {
            model.to_string()
        } else {
            "global".to_string()
        };

        let mut breakers = self.circuit_breakers.write();
        if let Some(breaker) = breakers.get_mut(&key) {
            breaker.record_success();
        }
    }

    /// Check rate limit
    fn check_rate_limit(&self, model: &str) -> Option<SimulationError> {
        // Simplified rate limiting - in production would use token bucket
        let limit = self.config.rate_limiting.get_limit(model);

        // Simple probabilistic rate limiting for simulation
        let mut rng = rand::thread_rng();
        let rate_limit_prob = 1.0 / limit.requests_per_minute as f64;

        if rng.gen::<f64>() < rate_limit_prob {
            return Some(SimulationError::RateLimitExceeded {
                retry_after: limit.retry_after(1000),
            });
        }

        None
    }

    /// Get circuit breaker status for a model
    pub fn circuit_breaker_status(&self, model: &str) -> Option<CircuitBreakerStatus> {
        let key = if self.config.circuit_breaker.per_model {
            model.to_string()
        } else {
            "global".to_string()
        };

        self.circuit_breakers.read()
            .get(&key)
            .map(|b| b.status())
    }

    /// Reset all circuit breakers
    pub fn reset_circuit_breakers(&self) {
        self.circuit_breakers.write().clear();
    }
}

impl Clone for ChaosEngine {
    fn clone(&self) -> Self {
        Self::new(self.config.clone())
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            opened_at: None,
        }
    }

    pub fn is_open(&mut self) -> bool {
        match self.state {
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(opened) = self.opened_at {
                    if opened.elapsed() >= self.config.recovery_timeout() {
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        return false;
                    }
                }
                true
            }
            CircuitState::HalfOpen => false,
            CircuitState::Closed => false,
        }
    }

    pub fn record_failure(&mut self) {
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;

                // Check if we should open
                if self.failure_count >= self.config.failure_threshold {
                    // Check if failures are within the window
                    if let Some(last) = self.last_failure {
                        if last.elapsed() <= self.config.failure_window() {
                            self.state = CircuitState::Open;
                            self.opened_at = Some(Instant::now());
                        } else {
                            // Reset if outside window
                            self.failure_count = 1;
                        }
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open reopens
                self.state = CircuitState::Open;
                self.opened_at = Some(Instant::now());
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.opened_at = None;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but ignore
            }
        }
    }

    pub fn status(&self) -> CircuitBreakerStatus {
        CircuitBreakerStatus {
            state: self.state,
            failure_count: self.failure_count,
            success_count: self.success_count,
            last_failure: self.last_failure,
            opened_at: self.opened_at,
        }
    }
}

/// Status information for a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<Instant>,
    pub opened_at: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_engine_disabled() {
        let config = ChaosConfig::default();
        let engine = ChaosEngine::new(config);

        assert!(!engine.is_active());
        assert!(engine.maybe_inject_error("gpt-4", "/chat/completions").is_none());
    }

    #[test]
    fn test_chaos_engine_enabled() {
        let mut config = ChaosConfig::default();
        config.enabled = true;
        config.global_probability = 1.0;
        config.errors = vec![ErrorInjectionRule {
            name: "always_fail".to_string(),
            error_type: InjectedErrorType::ServerError,
            probability: 1.0,
            models: vec![],
            endpoints: vec![],
            message: Some("Test error".to_string()),
            status_code: Some(500),
            delay_ms: None,
            enabled: true,
        }];

        let engine = ChaosEngine::new(config);
        assert!(engine.is_active());

        let error = engine.maybe_inject_error("gpt-4", "/chat/completions");
        assert!(error.is_some());
    }

    #[test]
    fn test_model_filter() {
        let mut config = ChaosConfig::default();
        config.enabled = true;
        config.global_probability = 1.0;
        config.errors = vec![ErrorInjectionRule {
            name: "gpt4_only".to_string(),
            error_type: InjectedErrorType::ServerError,
            probability: 1.0,
            models: vec!["gpt-4".to_string()],
            endpoints: vec![],
            message: None,
            status_code: None,
            delay_ms: None,
            enabled: true,
        }];

        let engine = ChaosEngine::new(config);

        // Should match gpt-4
        assert!(engine.maybe_inject_error("gpt-4", "/chat").is_some());
        assert!(engine.maybe_inject_error("gpt-4-turbo", "/chat").is_some());

        // Should not match claude
        assert!(engine.maybe_inject_error("claude-3", "/chat").is_none());
    }

    #[test]
    fn test_circuit_breaker() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            failure_window_secs: 60,
            recovery_timeout_secs: 1,
            success_threshold: 2,
            per_model: false,
        };

        let mut breaker = CircuitBreaker::new(config);

        // Initially closed
        assert!(!breaker.is_open());

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert!(!breaker.is_open());

        breaker.record_failure();
        assert!(breaker.is_open());
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 1,
            failure_window_secs: 60,
            recovery_timeout_secs: 0, // Immediate recovery for test
            success_threshold: 1,
            per_model: false,
        };

        let mut breaker = CircuitBreaker::new(config);

        // Initially closed
        assert_eq!(breaker.state, CircuitState::Closed);

        // Record failure to open the breaker
        breaker.record_failure();
        // The breaker should be open now (threshold=1)
        assert_eq!(breaker.state, CircuitState::Open);

        // After recovery timeout (0 seconds), is_open() should return false
        // and transition to half-open
        std::thread::sleep(std::time::Duration::from_millis(10));
        let is_open = breaker.is_open();
        assert!(!is_open); // Now half-open
        assert_eq!(breaker.state, CircuitState::HalfOpen);

        // Success should close it
        breaker.record_success();
        assert_eq!(breaker.state, CircuitState::Closed);
    }
}
