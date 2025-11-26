//! Configuration module for LLM-Simulator
//!
//! Provides hierarchical configuration with support for:
//! - YAML/TOML/JSON config files
//! - Environment variable overrides
//! - Runtime reconfiguration
//! - Validation

mod models;
mod latency;
mod chaos;
pub mod security;

pub use models::*;
pub use latency::*;
pub use chaos::*;
pub use security::SecurityConfig;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use crate::error::{SimulationError, SimulatorResult};
use crate::types::Provider;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulatorConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Model configurations by provider
    pub models: HashMap<String, ModelConfig>,
    /// Latency simulation settings
    pub latency: LatencyConfig,
    /// Chaos engineering settings
    pub chaos: ChaosConfig,
    /// Telemetry settings
    pub telemetry: TelemetryConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Default provider
    #[serde(default)]
    pub default_provider: Provider,
    /// Seed for deterministic behavior (None = random)
    pub seed: Option<u64>,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        let mut models = HashMap::new();

        // Default OpenAI models
        models.insert("gpt-4".to_string(), ModelConfig::gpt4());
        models.insert("gpt-4-turbo".to_string(), ModelConfig::gpt4_turbo());
        models.insert("gpt-4o".to_string(), ModelConfig::gpt4o());
        models.insert("gpt-4o-mini".to_string(), ModelConfig::gpt4o_mini());
        models.insert("gpt-3.5-turbo".to_string(), ModelConfig::gpt35_turbo());

        // Default Anthropic models
        models.insert("claude-3-5-sonnet-20241022".to_string(), ModelConfig::claude_35_sonnet());
        models.insert("claude-3-opus-20240229".to_string(), ModelConfig::claude_3_opus());
        models.insert("claude-3-sonnet-20240229".to_string(), ModelConfig::claude_3_sonnet());
        models.insert("claude-3-haiku-20240307".to_string(), ModelConfig::claude_3_haiku());

        // Default Google models
        models.insert("gemini-1.5-pro".to_string(), ModelConfig::gemini_15_pro());
        models.insert("gemini-1.5-flash".to_string(), ModelConfig::gemini_15_flash());

        // Default embedding models
        models.insert("text-embedding-ada-002".to_string(), ModelConfig::embedding_ada());
        models.insert("text-embedding-3-small".to_string(), ModelConfig::embedding_3_small());
        models.insert("text-embedding-3-large".to_string(), ModelConfig::embedding_3_large());

        Self {
            server: ServerConfig::default(),
            models,
            latency: LatencyConfig::default(),
            chaos: ChaosConfig::default(),
            telemetry: TelemetryConfig::default(),
            security: SecurityConfig::default(),
            default_provider: Provider::OpenAI,
            seed: None,
        }
    }
}

impl SimulatorConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> SimulatorResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| SimulationError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| SimulationError::Config(format!("YAML parse error: {}", e)))?,
            Some("toml") => toml::from_str(&content)
                .map_err(|e| SimulationError::Config(format!("TOML parse error: {}", e)))?,
            Some("json") => serde_json::from_str(&content)
                .map_err(|e| SimulationError::Config(format!("JSON parse error: {}", e)))?,
            _ => return Err(SimulationError::Config(
                "Unsupported config file format. Use .yaml, .toml, or .json".to_string()
            )),
        };

        config.validate()?;
        Ok(config)
    }

    /// Load configuration with environment variable overrides
    pub fn from_env() -> SimulatorResult<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(port) = std::env::var("LLM_SIMULATOR_PORT") {
            config.server.port = port.parse().map_err(|_| {
                SimulationError::Config("Invalid port number".to_string())
            })?;
        }

        if let Ok(host) = std::env::var("LLM_SIMULATOR_HOST") {
            config.server.host = host;
        }

        if let Ok(seed) = std::env::var("LLM_SIMULATOR_SEED") {
            config.seed = Some(seed.parse().map_err(|_| {
                SimulationError::Config("Invalid seed value".to_string())
            })?);
        }

        if let Ok(val) = std::env::var("LLM_SIMULATOR_CHAOS_ENABLED") {
            config.chaos.enabled = val.parse().unwrap_or(false);
        }

        if let Ok(val) = std::env::var("LLM_SIMULATOR_LATENCY_ENABLED") {
            config.latency.enabled = val.parse().unwrap_or(true);
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> SimulatorResult<()> {
        self.server.validate()?;
        self.latency.validate()?;
        self.chaos.validate()?;

        for (name, model) in &self.models {
            model.validate().map_err(|e| {
                SimulationError::Config(format!("Invalid model config '{}': {}", name, e))
            })?;
        }

        Ok(())
    }

    /// Get model configuration by name
    pub fn get_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.get(name)
    }

    /// Add or update a model configuration
    pub fn set_model(&mut self, name: String, config: ModelConfig) {
        self.models.insert(name, config);
    }

    /// Create a minimal configuration with only essential models
    pub fn minimal() -> Self {
        let mut models = HashMap::new();
        models.insert("gpt-4".to_string(), ModelConfig::gpt4());
        models.insert("text-embedding-ada-002".to_string(), ModelConfig::embedding_ada());

        Self {
            server: ServerConfig::default(),
            models,
            latency: LatencyConfig {
                enabled: false,
                ..Default::default()
            },
            chaos: ChaosConfig::default(),
            telemetry: TelemetryConfig::default(),
            security: SecurityConfig::default(),
            default_provider: Provider::OpenAI,
            seed: None,
        }
    }

    /// Create a production-ready configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.security.api_keys.enabled = true;
        config.security.admin.require_admin_key = true;
        config.telemetry.json_logs = true;
        config.telemetry.trace_requests = true;
        config
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Request timeout
    #[serde(with = "humantime_serde")]
    pub request_timeout: Duration,
    /// Request timeout in seconds (alternative to request_timeout)
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    /// Enable CORS
    pub cors_enabled: bool,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Enable request logging
    pub request_logging: bool,
    /// Worker threads (0 = auto)
    pub worker_threads: usize,
}

fn default_request_timeout_secs() -> u64 {
    300
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            max_concurrent_requests: 10_000,
            request_timeout: Duration::from_secs(300),
            request_timeout_secs: 300,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            request_logging: true,
            worker_threads: 0,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> SimulatorResult<()> {
        if self.port == 0 {
            return Err(SimulationError::Validation {
                message: "Port cannot be 0".to_string(),
                param: Some("server.port".to_string()),
            });
        }
        if self.max_concurrent_requests == 0 {
            return Err(SimulationError::Validation {
                message: "max_concurrent_requests must be greater than 0".to_string(),
                param: Some("server.max_concurrent_requests".to_string()),
            });
        }
        Ok(())
    }

    /// Get the socket address
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryConfig {
    /// Enable telemetry
    pub enabled: bool,
    /// Log level
    pub log_level: String,
    /// Enable JSON logging
    pub json_logs: bool,
    /// OpenTelemetry endpoint
    pub otlp_endpoint: Option<String>,
    /// Prometheus metrics endpoint path
    pub metrics_path: String,
    /// Service name for tracing
    pub service_name: String,
    /// Enable request tracing
    pub trace_requests: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            json_logs: false,
            otlp_endpoint: None,
            metrics_path: "/metrics".to_string(),
            service_name: "llm-simulator".to_string(),
            trace_requests: true,
        }
    }
}

/// Helper module for Duration serialization
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}s", duration.as_secs()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_duration(&s).map_err(serde::de::Error::custom)
    }

    fn parse_duration(s: &str) -> Result<Duration, String> {
        let s = s.trim();
        if let Some(secs) = s.strip_suffix('s') {
            secs.trim().parse::<u64>()
                .map(Duration::from_secs)
                .map_err(|_| format!("Invalid duration: {}", s))
        } else if let Some(millis) = s.strip_suffix("ms") {
            millis.trim().parse::<u64>()
                .map(Duration::from_millis)
                .map_err(|_| format!("Invalid duration: {}", s))
        } else if let Some(mins) = s.strip_suffix('m') {
            mins.trim().parse::<u64>()
                .map(|m| Duration::from_secs(m * 60))
                .map_err(|_| format!("Invalid duration: {}", s))
        } else {
            s.parse::<u64>()
                .map(Duration::from_secs)
                .map_err(|_| format!("Invalid duration: {}", s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SimulatorConfig::default();
        assert_eq!(config.server.port, 8080);
        assert!(config.models.contains_key("gpt-4"));
        assert!(config.models.contains_key("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_config_validation() {
        let config = SimulatorConfig::default();
        let result = config.validate();
        if let Err(e) = &result {
            panic!("Validation failed: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_model() {
        let config = SimulatorConfig::default();
        let model = config.get_model("gpt-4");
        assert!(model.is_some());
        assert!(model.unwrap().supports_streaming);
    }

    #[test]
    fn test_invalid_port() {
        let mut config = SimulatorConfig::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }
}
