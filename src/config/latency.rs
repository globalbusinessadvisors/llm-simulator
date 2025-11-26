//! Latency simulation configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::error::{SimulationError, SimulatorResult};

/// Latency simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LatencyConfig {
    /// Enable latency simulation
    pub enabled: bool,
    /// Global latency multiplier (1.0 = normal)
    pub multiplier: f64,
    /// Default latency profile
    pub default_profile: String,
    /// Named latency profiles
    pub profiles: HashMap<String, LatencyProfile>,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();

        // Fast profile (local/edge)
        profiles.insert("fast".to_string(), LatencyProfile {
            ttft: LatencyDistribution::normal(50.0, 10.0),
            itl: LatencyDistribution::normal(15.0, 3.0),
            overhead: Duration::from_millis(5),
        });

        // Standard profile (typical cloud)
        profiles.insert("standard".to_string(), LatencyProfile {
            ttft: LatencyDistribution::normal(200.0, 50.0),
            itl: LatencyDistribution::normal(30.0, 8.0),
            overhead: Duration::from_millis(10),
        });

        // Slow profile (congested/distant)
        profiles.insert("slow".to_string(), LatencyProfile {
            ttft: LatencyDistribution::normal(500.0, 100.0),
            itl: LatencyDistribution::normal(60.0, 15.0),
            overhead: Duration::from_millis(20),
        });

        // GPT-4 realistic profile
        profiles.insert("gpt4".to_string(), LatencyProfile {
            ttft: LatencyDistribution::log_normal(300.0, 150.0),
            itl: LatencyDistribution::log_normal(40.0, 15.0),
            overhead: Duration::from_millis(15),
        });

        // Claude realistic profile
        profiles.insert("claude".to_string(), LatencyProfile {
            ttft: LatencyDistribution::log_normal(250.0, 100.0),
            itl: LatencyDistribution::log_normal(35.0, 12.0),
            overhead: Duration::from_millis(12),
        });

        // Gemini realistic profile
        profiles.insert("gemini".to_string(), LatencyProfile {
            ttft: LatencyDistribution::log_normal(200.0, 80.0),
            itl: LatencyDistribution::log_normal(25.0, 10.0),
            overhead: Duration::from_millis(10),
        });

        // Zero latency (for testing)
        profiles.insert("instant".to_string(), LatencyProfile {
            ttft: LatencyDistribution::fixed(0.0),
            itl: LatencyDistribution::fixed(0.0),
            overhead: Duration::from_millis(0),
        });

        Self {
            enabled: true,
            multiplier: 1.0,
            default_profile: "standard".to_string(),
            profiles,
        }
    }
}

impl LatencyConfig {
    pub fn validate(&self) -> SimulatorResult<()> {
        if self.multiplier < 0.0 {
            return Err(SimulationError::Validation {
                message: "latency multiplier cannot be negative".to_string(),
                param: Some("latency.multiplier".to_string()),
            });
        }
        if !self.profiles.contains_key(&self.default_profile) {
            return Err(SimulationError::Validation {
                message: format!("default profile '{}' not found", self.default_profile),
                param: Some("latency.default_profile".to_string()),
            });
        }
        for (name, profile) in &self.profiles {
            profile.validate().map_err(|e| SimulationError::Validation {
                message: format!("Invalid latency profile '{}': {}", name, e),
                param: Some(format!("latency.profiles.{}", name)),
            })?;
        }
        Ok(())
    }

    /// Get a latency profile by name
    pub fn get_profile(&self, name: &str) -> Option<&LatencyProfile> {
        self.profiles.get(name)
    }

    /// Get the default latency profile
    pub fn default_profile(&self) -> &LatencyProfile {
        self.profiles.get(&self.default_profile)
            .expect("Default profile should always exist")
    }
}

/// A latency profile defining timing characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyProfile {
    /// Time to First Token distribution (milliseconds)
    pub ttft: LatencyDistribution,
    /// Inter-Token Latency distribution (milliseconds)
    pub itl: LatencyDistribution,
    /// Fixed overhead per request
    #[serde(with = "duration_millis")]
    pub overhead: Duration,
}

impl LatencyProfile {
    pub fn validate(&self) -> Result<(), String> {
        self.ttft.validate()?;
        self.itl.validate()?;
        Ok(())
    }
}

/// Statistical distribution for latency values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LatencyDistribution {
    /// Fixed/constant latency
    Fixed { value_ms: f64 },
    /// Normal (Gaussian) distribution
    Normal { mean_ms: f64, std_dev_ms: f64 },
    /// Log-normal distribution (more realistic for network latency)
    LogNormal { mean_ms: f64, std_dev_ms: f64 },
    /// Uniform distribution between min and max
    Uniform { min_ms: f64, max_ms: f64 },
    /// Exponential distribution
    Exponential { mean_ms: f64 },
    /// Pareto distribution (for modeling tail latency)
    Pareto { scale_ms: f64, shape: f64 },
}

impl LatencyDistribution {
    pub fn fixed(value_ms: f64) -> Self {
        Self::Fixed { value_ms }
    }

    pub fn normal(mean_ms: f64, std_dev_ms: f64) -> Self {
        Self::Normal { mean_ms, std_dev_ms }
    }

    pub fn log_normal(mean_ms: f64, std_dev_ms: f64) -> Self {
        Self::LogNormal { mean_ms, std_dev_ms }
    }

    pub fn uniform(min_ms: f64, max_ms: f64) -> Self {
        Self::Uniform { min_ms, max_ms }
    }

    pub fn exponential(mean_ms: f64) -> Self {
        Self::Exponential { mean_ms }
    }

    pub fn pareto(scale_ms: f64, shape: f64) -> Self {
        Self::Pareto { scale_ms, shape }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Fixed { value_ms } if *value_ms < 0.0 => {
                Err("Fixed latency cannot be negative".to_string())
            }
            Self::Normal { std_dev_ms, .. } if *std_dev_ms < 0.0 => {
                Err("Standard deviation cannot be negative".to_string())
            }
            Self::LogNormal { std_dev_ms, .. } if *std_dev_ms <= 0.0 => {
                Err("Log-normal std_dev must be positive".to_string())
            }
            Self::Uniform { min_ms, max_ms } if min_ms > max_ms => {
                Err("Uniform min cannot be greater than max".to_string())
            }
            Self::Exponential { mean_ms } if *mean_ms <= 0.0 => {
                Err("Exponential mean must be positive".to_string())
            }
            Self::Pareto { scale_ms, shape } if *scale_ms <= 0.0 || *shape <= 0.0 => {
                Err("Pareto scale and shape must be positive".to_string())
            }
            _ => Ok(()),
        }
    }

    /// Get the expected/mean value of this distribution
    pub fn mean(&self) -> f64 {
        match self {
            Self::Fixed { value_ms } => *value_ms,
            Self::Normal { mean_ms, .. } => *mean_ms,
            Self::LogNormal { mean_ms, .. } => *mean_ms,
            Self::Uniform { min_ms, max_ms } => (min_ms + max_ms) / 2.0,
            Self::Exponential { mean_ms } => *mean_ms,
            Self::Pareto { scale_ms, shape } if *shape > 1.0 => {
                (shape * scale_ms) / (shape - 1.0)
            }
            Self::Pareto { .. } => f64::INFINITY,
        }
    }
}

impl Default for LatencyDistribution {
    fn default() -> Self {
        Self::Normal {
            mean_ms: 100.0,
            std_dev_ms: 20.0,
        }
    }
}

/// Helper module for Duration serialization in milliseconds
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LatencyConfig::default();
        assert!(config.enabled);
        assert!(config.profiles.contains_key("standard"));
        assert!(config.profiles.contains_key("fast"));
        assert!(config.profiles.contains_key("slow"));
    }

    #[test]
    fn test_config_validation() {
        let config = LatencyConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_distribution_validation() {
        assert!(LatencyDistribution::fixed(100.0).validate().is_ok());
        assert!(LatencyDistribution::fixed(-1.0).validate().is_err());
        assert!(LatencyDistribution::normal(100.0, 20.0).validate().is_ok());
        assert!(LatencyDistribution::normal(100.0, -5.0).validate().is_err());
    }

    #[test]
    fn test_distribution_mean() {
        assert_eq!(LatencyDistribution::fixed(50.0).mean(), 50.0);
        assert_eq!(LatencyDistribution::normal(100.0, 20.0).mean(), 100.0);
        assert_eq!(LatencyDistribution::uniform(0.0, 100.0).mean(), 50.0);
    }

    #[test]
    fn test_get_profile() {
        let config = LatencyConfig::default();
        let profile = config.get_profile("fast");
        assert!(profile.is_some());
        assert!(config.get_profile("nonexistent").is_none());
    }
}
