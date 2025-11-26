//! Security configuration for LLM-Simulator
//!
//! Provides configuration for:
//! - API key authentication
//! - Rate limiting
//! - CORS settings
//! - Security headers

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// API key authentication settings
    pub api_keys: ApiKeyConfig,
    /// Admin authorization settings
    pub admin: AdminConfig,
    /// CORS settings
    pub cors: CorsConfig,
    /// Rate limiting settings
    pub rate_limiting: RateLimitConfig,
    /// Security headers settings
    pub headers: SecurityHeadersConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            api_keys: ApiKeyConfig::default(),
            admin: AdminConfig::default(),
            cors: CorsConfig::default(),
            rate_limiting: RateLimitConfig::default(),
            headers: SecurityHeadersConfig::default(),
        }
    }
}

impl SecurityConfig {
    /// Validate the security configuration
    pub fn validate(&self) -> Result<(), String> {
        self.api_keys.validate()?;
        self.cors.validate()?;
        self.rate_limiting.validate()?;
        Ok(())
    }
}

/// API key authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiKeyConfig {
    /// Enable API key authentication
    pub enabled: bool,
    /// List of valid API keys
    pub keys: Vec<ApiKeyEntry>,
    /// Header name for API key (default: Authorization)
    pub header_name: String,
    /// Whether to allow requests without API key (for health endpoints)
    pub allow_anonymous_health: bool,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for development
            keys: Vec::new(),
            header_name: "Authorization".to_string(),
            allow_anonymous_health: true,
        }
    }
}

impl ApiKeyConfig {
    /// Validate the API key configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.keys.is_empty() {
            return Err("API key authentication is enabled but no keys are configured".to_string());
        }

        // Check for duplicate key IDs
        let mut ids = HashSet::new();
        for key in &self.keys {
            if !ids.insert(&key.id) {
                return Err(format!("Duplicate API key ID: {}", key.id));
            }
        }

        Ok(())
    }

    /// Find a key by its value
    pub fn find_key(&self, key_value: &str) -> Option<&ApiKeyEntry> {
        self.keys.iter().find(|k| k.key == key_value)
    }
}

/// Individual API key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// Unique identifier for this key
    pub id: String,
    /// The actual API key value
    pub key: String,
    /// Role associated with this key (admin, user, readonly)
    #[serde(default)]
    pub role: ApiKeyRole,
    /// Rate limit tier for this key
    #[serde(default)]
    pub rate_limit_tier: RateLimitTier,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether this key is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// API key roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyRole {
    /// Full admin access
    Admin,
    /// Standard user access
    #[default]
    User,
    /// Read-only access
    Readonly,
}

impl std::fmt::Display for ApiKeyRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::User => write!(f, "user"),
            Self::Readonly => write!(f, "readonly"),
        }
    }
}

/// Rate limit tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitTier {
    /// Standard tier (default limits)
    #[default]
    Standard,
    /// Premium tier (higher limits)
    Premium,
    /// Admin tier (highest limits)
    Admin,
    /// Unlimited (no rate limiting)
    Unlimited,
}

impl std::fmt::Display for RateLimitTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::Premium => write!(f, "premium"),
            Self::Admin => write!(f, "admin"),
            Self::Unlimited => write!(f, "unlimited"),
        }
    }
}

/// Admin authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    /// Require admin role for admin endpoints
    pub require_admin_key: bool,
    /// IP addresses allowed to access admin endpoints (empty = all allowed)
    pub ip_allowlist: Vec<IpAddr>,
    /// Log all admin access attempts
    pub log_access: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            require_admin_key: true,
            ip_allowlist: Vec::new(),
            log_access: true,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,
    /// Allowed origins (use "*" for any, not recommended in production)
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers
    pub exposed_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight cache (in seconds)
    pub max_age_seconds: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()], // Permissive by default for dev
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
                "X-API-Key".to_string(),
            ],
            exposed_headers: vec![
                "X-Request-ID".to_string(),
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "X-RateLimit-Reset".to_string(),
            ],
            allow_credentials: false,
            max_age_seconds: 3600,
        }
    }
}

impl CorsConfig {
    /// Validate CORS configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.allowed_origins.is_empty() {
            return Err("CORS is enabled but no allowed origins are configured".to_string());
        }

        // Warn about wildcard in production (via log, not error)
        if self.allowed_origins.iter().any(|o| o == "*") && self.allow_credentials {
            return Err("Cannot use wildcard origin '*' with allow_credentials=true".to_string());
        }

        Ok(())
    }

    /// Check if an origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if !self.enabled {
            return true;
        }

        self.allowed_origins.iter().any(|allowed| {
            if allowed == "*" {
                true
            } else if allowed.starts_with("*.") {
                // Wildcard subdomain matching
                let domain = &allowed[2..];
                origin.ends_with(domain) || origin == &allowed[2..]
            } else {
                origin == allowed
            }
        })
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Default tier for unauthenticated requests
    pub default_tier: RateLimitTier,
    /// Tier configurations
    pub tiers: RateLimitTiers,
    /// Clean up inactive buckets after this duration
    #[serde(with = "humantime_serde_duration")]
    pub bucket_ttl: Duration,
    /// How often to clean up expired buckets
    #[serde(with = "humantime_serde_duration")]
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_tier: RateLimitTier::Standard,
            tiers: RateLimitTiers::default(),
            bucket_ttl: Duration::from_secs(3600),
            cleanup_interval: Duration::from_secs(300),
        }
    }
}

impl RateLimitConfig {
    /// Validate rate limit configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.tiers.standard.requests_per_minute == 0 {
            return Err("Standard tier requests_per_minute cannot be 0".to_string());
        }
        Ok(())
    }

    /// Get the tier configuration
    pub fn get_tier_config(&self, tier: RateLimitTier) -> &TierConfig {
        match tier {
            RateLimitTier::Standard => &self.tiers.standard,
            RateLimitTier::Premium => &self.tiers.premium,
            RateLimitTier::Admin => &self.tiers.admin,
            RateLimitTier::Unlimited => &self.tiers.unlimited,
        }
    }
}

/// Rate limit tier configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitTiers {
    pub standard: TierConfig,
    pub premium: TierConfig,
    pub admin: TierConfig,
    pub unlimited: TierConfig,
}

impl Default for RateLimitTiers {
    fn default() -> Self {
        Self {
            standard: TierConfig {
                requests_per_minute: 60,
                burst_size: 10,
            },
            premium: TierConfig {
                requests_per_minute: 600,
                burst_size: 100,
            },
            admin: TierConfig {
                requests_per_minute: 1000,
                burst_size: 200,
            },
            unlimited: TierConfig {
                requests_per_minute: u32::MAX,
                burst_size: u32::MAX,
            },
        }
    }
}

/// Configuration for a single rate limit tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum burst size (tokens in bucket)
    pub burst_size: u32,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityHeadersConfig {
    /// Enable security headers
    pub enabled: bool,
    /// Enable HSTS (Strict-Transport-Security)
    pub hsts_enabled: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// HSTS preload
    pub hsts_preload: bool,
    /// Content Security Policy
    pub content_security_policy: Option<String>,
    /// X-Frame-Options value
    pub frame_options: String,
    /// Referrer-Policy value
    pub referrer_policy: String,
    /// Permissions-Policy value
    pub permissions_policy: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hsts_enabled: false, // Disabled by default, enable when behind TLS
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            content_security_policy: Some("default-src 'none'; frame-ancestors 'none'".to_string()),
            frame_options: "DENY".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
        }
    }
}

/// Helper module for Duration serialization
mod humantime_serde_duration {
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
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert!(!config.api_keys.enabled);
        assert!(config.cors.enabled);
        assert!(config.rate_limiting.enabled);
        assert!(config.headers.enabled);
    }

    #[test]
    fn test_api_key_validation() {
        let mut config = ApiKeyConfig::default();
        config.enabled = true;
        assert!(config.validate().is_err());

        config.keys.push(ApiKeyEntry {
            id: "key-1".to_string(),
            key: "test-key".to_string(),
            role: ApiKeyRole::User,
            rate_limit_tier: RateLimitTier::Standard,
            description: None,
            enabled: true,
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_cors_origin_matching() {
        let config = CorsConfig {
            enabled: true,
            allowed_origins: vec![
                "https://example.com".to_string(),
                "*.example.org".to_string(),
            ],
            ..Default::default()
        };

        assert!(config.is_origin_allowed("https://example.com"));
        assert!(config.is_origin_allowed("https://sub.example.org"));
        assert!(!config.is_origin_allowed("https://evil.com"));
    }

    #[test]
    fn test_rate_limit_tiers() {
        let config = RateLimitConfig::default();

        assert_eq!(config.get_tier_config(RateLimitTier::Standard).requests_per_minute, 60);
        assert_eq!(config.get_tier_config(RateLimitTier::Premium).requests_per_minute, 600);
        assert_eq!(config.get_tier_config(RateLimitTier::Admin).requests_per_minute, 1000);
    }

    #[test]
    fn test_api_key_role_display() {
        assert_eq!(ApiKeyRole::Admin.to_string(), "admin");
        assert_eq!(ApiKeyRole::User.to_string(), "user");
        assert_eq!(ApiKeyRole::Readonly.to_string(), "readonly");
    }
}
