//! Security Middleware Stack
//!
//! Combines all security middleware into a cohesive stack
//! with proper ordering and configuration.

use std::sync::Arc;
use axum::{
    Router,
    middleware as axum_middleware,
};
use tower_http::cors::{CorsLayer, Any, AllowOrigin};

use crate::config::security::{SecurityConfig, CorsConfig};

use super::{
    api_key::{api_key_auth_middleware, admin_auth_middleware},
    rate_limit::{rate_limit_middleware, RateLimiter},
    headers::security_headers_middleware,
};

/// Security state containing all security-related components
#[derive(Clone)]
pub struct SecurityState {
    pub api_key_config: Arc<crate::config::security::ApiKeyConfig>,
    pub admin_config: Arc<crate::config::security::AdminConfig>,
    pub rate_limiter: Arc<RateLimiter>,
    pub headers_config: Arc<crate::config::security::SecurityHeadersConfig>,
    pub cors_config: Arc<CorsConfig>,
}

impl SecurityState {
    /// Create security state from configuration
    pub fn new(config: &SecurityConfig) -> Self {
        let rate_limiter = RateLimiter::new(Arc::new(config.rate_limiting.clone()));

        Self {
            api_key_config: Arc::new(config.api_keys.clone()),
            admin_config: Arc::new(config.admin.clone()),
            rate_limiter: Arc::new(rate_limiter),
            headers_config: Arc::new(config.headers.clone()),
            cors_config: Arc::new(config.cors.clone()),
        }
    }

    /// Create a default security state (for development)
    pub fn development() -> Self {
        Self::new(&SecurityConfig::default())
    }
}

/// Build CORS layer from configuration
pub fn build_cors_layer(config: &CorsConfig) -> CorsLayer {
    if !config.enabled {
        return CorsLayer::very_permissive();
    }

    let mut cors = CorsLayer::new();

    // Configure allowed origins
    if config.allowed_origins.iter().any(|o| o == "*") {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<_> = config.allowed_origins.iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(AllowOrigin::list(origins));
    }

    // Configure allowed methods
    let methods: Vec<_> = config.allowed_methods.iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);

    // Configure allowed headers
    let headers: Vec<_> = config.allowed_headers.iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.allow_headers(headers);

    // Configure exposed headers
    let exposed: Vec<_> = config.exposed_headers.iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.expose_headers(exposed);

    // Configure credentials
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    // Configure max age
    cors = cors.max_age(std::time::Duration::from_secs(config.max_age_seconds));

    cors
}

/// Apply security middleware to a router
///
/// Middleware is applied in the correct order:
/// 1. Security headers (outermost - applied to all responses)
/// 2. CORS (handle preflight requests)
/// 3. Rate limiting (applied before expensive operations)
/// 4. API key authentication (innermost - applied to requests)
pub fn apply_security_middleware<S>(
    router: Router<S>,
    security: SecurityState,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let cors = build_cors_layer(&security.cors_config);

    router
        // Apply in reverse order (innermost first in code = outermost in execution)
        // Admin auth check (after API key auth sets the key info)
        .layer(axum_middleware::from_fn_with_state(
            security.admin_config.clone(),
            admin_auth_middleware,
        ))
        // API key authentication
        .layer(axum_middleware::from_fn_with_state(
            security.api_key_config.clone(),
            api_key_auth_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            security.rate_limiter.clone(),
            rate_limit_middleware,
        ))
        .layer(cors)
        .layer(axum_middleware::from_fn_with_state(
            security.headers_config.clone(),
            security_headers_middleware,
        ))
}

/// Create a minimal security configuration for testing
pub fn test_security_config() -> SecurityConfig {
    SecurityConfig {
        api_keys: crate::config::security::ApiKeyConfig {
            enabled: false,
            ..Default::default()
        },
        rate_limiting: crate::config::security::RateLimitConfig {
            enabled: false,
            ..Default::default()
        },
        headers: crate::config::security::SecurityHeadersConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Create a production security configuration
pub fn production_security_config() -> SecurityConfig {
    SecurityConfig {
        api_keys: crate::config::security::ApiKeyConfig {
            enabled: true,
            allow_anonymous_health: true,
            ..Default::default()
        },
        admin: crate::config::security::AdminConfig {
            require_admin_key: true,
            log_access: true,
            ..Default::default()
        },
        cors: CorsConfig {
            enabled: true,
            allowed_origins: vec![], // Must be configured
            allow_credentials: false,
            ..Default::default()
        },
        rate_limiting: crate::config::security::RateLimitConfig {
            enabled: true,
            ..Default::default()
        },
        headers: crate::config::security::SecurityHeadersConfig {
            enabled: true,
            hsts_enabled: true,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_state_creation() {
        let config = SecurityConfig::default();
        let state = SecurityState::new(&config);

        assert!(!state.api_key_config.enabled);
        assert!(state.rate_limiter.is_enabled());
    }

    #[test]
    fn test_build_cors_layer_permissive() {
        let config = CorsConfig::default();
        let _layer = build_cors_layer(&config);
        // Layer creation should not panic
    }

    #[test]
    fn test_build_cors_layer_restrictive() {
        let config = CorsConfig {
            enabled: true,
            allowed_origins: vec![
                "https://example.com".to_string(),
                "https://api.example.com".to_string(),
            ],
            allow_credentials: false,
            ..Default::default()
        };
        let _layer = build_cors_layer(&config);
        // Layer creation should not panic
    }

    #[test]
    fn test_test_security_config() {
        let config = test_security_config();
        assert!(!config.api_keys.enabled);
        assert!(!config.rate_limiting.enabled);
        assert!(!config.headers.enabled);
    }

    #[test]
    fn test_production_security_config() {
        let config = production_security_config();
        assert!(config.api_keys.enabled);
        assert!(config.rate_limiting.enabled);
        assert!(config.headers.enabled);
        assert!(config.headers.hsts_enabled);
    }
}
