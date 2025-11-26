//! API Key Authentication
//!
//! Provides secure API key validation with support for:
//! - Bearer token authentication
//! - Role-based access control
//! - Key rotation without restart

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tracing::{info, warn};

use crate::config::security::{ApiKeyConfig, ApiKeyEntry, ApiKeyRole};
use crate::error::ErrorResponse;

/// Extracted API key information attached to request extensions
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    /// Key ID
    pub id: String,
    /// Role associated with this key
    pub role: ApiKeyRole,
    /// Rate limit tier
    pub tier: crate::config::security::RateLimitTier,
    /// Whether this is an anonymous request (no key provided)
    pub anonymous: bool,
}

impl ApiKeyInfo {
    /// Create an anonymous key info for health endpoints
    pub fn anonymous() -> Self {
        Self {
            id: "anonymous".to_string(),
            role: ApiKeyRole::Readonly,
            tier: crate::config::security::RateLimitTier::Standard,
            anonymous: true,
        }
    }

    /// Create from an API key entry
    pub fn from_entry(entry: &ApiKeyEntry) -> Self {
        Self {
            id: entry.id.clone(),
            role: entry.role,
            tier: entry.rate_limit_tier,
            anonymous: false,
        }
    }

    /// Check if this key has admin role
    pub fn is_admin(&self) -> bool {
        self.role == ApiKeyRole::Admin
    }

    /// Check if this key can write (admin or user)
    pub fn can_write(&self) -> bool {
        matches!(self.role, ApiKeyRole::Admin | ApiKeyRole::User)
    }
}

/// API Key authentication error
#[derive(Debug, Clone)]
pub enum AuthError {
    /// No authorization header provided
    MissingHeader,
    /// Invalid authorization format (not Bearer)
    InvalidFormat,
    /// API key not found or disabled
    InvalidKey,
    /// Key is disabled
    KeyDisabled,
    /// Insufficient permissions
    InsufficientPermissions { required: ApiKeyRole, actual: ApiKeyRole },
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            Self::MissingHeader => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "Missing Authorization header. Use 'Authorization: Bearer <api-key>'",
            ),
            Self::InvalidFormat => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "Invalid Authorization format. Use 'Bearer <api-key>'",
            ),
            Self::InvalidKey => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "Invalid API key",
            ),
            Self::KeyDisabled => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "API key is disabled",
            ),
            Self::InsufficientPermissions { required: _, actual: _ } => (
                StatusCode::FORBIDDEN,
                "permission_error",
                "Insufficient permissions",
            ),
        };

        let body = ErrorResponse::new(error_type, message);
        (status, Json(body)).into_response()
    }
}

/// Extract API key from Authorization header
pub fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| {
            if auth.starts_with("Bearer ") {
                Some(auth.trim_start_matches("Bearer ").to_string())
            } else if auth.starts_with("bearer ") {
                Some(auth.trim_start_matches("bearer ").to_string())
            } else if !auth.contains(' ') {
                // Plain key without Bearer prefix (for x-api-key header)
                Some(auth.to_string())
            } else {
                None
            }
        })
}

/// Check if a path is a health/metrics endpoint that should be exempt from auth
pub fn is_health_endpoint(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/healthz" | "/ready" | "/readyz" | "/metrics" | "/" | "/version"
    )
}

/// Check if a path is an admin endpoint
pub fn is_admin_endpoint(path: &str) -> bool {
    path.starts_with("/admin")
}

/// API Key authentication middleware
pub async fn api_key_auth_middleware(
    State(config): State<Arc<ApiKeyConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let path = request.uri().path();

    // Skip auth for health endpoints if configured
    if config.allow_anonymous_health && is_health_endpoint(path) {
        request.extensions_mut().insert(ApiKeyInfo::anonymous());
        return Ok(next.run(request).await);
    }

    // Skip if auth is disabled
    if !config.enabled {
        request.extensions_mut().insert(ApiKeyInfo::anonymous());
        return Ok(next.run(request).await);
    }

    // Extract API key from headers
    let api_key = extract_api_key(request.headers())
        .ok_or(AuthError::MissingHeader)?;

    // Validate the key
    let key_entry = config.find_key(&api_key)
        .ok_or_else(|| {
            // Log with only key prefix for security
            let key_prefix = if api_key.len() > 8 {
                &api_key[..8]
            } else {
                &api_key
            };
            warn!(
                key_prefix = %key_prefix,
                path = %path,
                "Invalid API key attempt"
            );
            AuthError::InvalidKey
        })?;

    // Check if key is enabled
    if !key_entry.enabled {
        warn!(
            key_id = %key_entry.id,
            path = %path,
            "Disabled API key used"
        );
        return Err(AuthError::KeyDisabled);
    }

    // Create key info and attach to request
    let key_info = ApiKeyInfo::from_entry(key_entry);

    info!(
        key_id = %key_info.id,
        role = %key_info.role,
        path = %path,
        "API key authenticated"
    );

    request.extensions_mut().insert(key_info);

    Ok(next.run(request).await)
}

/// Admin authorization middleware (must be applied after api_key_auth_middleware)
pub async fn admin_auth_middleware(
    State(config): State<Arc<crate::config::security::AdminConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let path = request.uri().path();

    // Only apply to admin endpoints
    if !is_admin_endpoint(path) {
        return Ok(next.run(request).await);
    }

    // Check if admin auth is required
    if !config.require_admin_key {
        return Ok(next.run(request).await);
    }

    // Get key info from extensions
    let key_info = request.extensions().get::<ApiKeyInfo>()
        .cloned()
        .unwrap_or_else(ApiKeyInfo::anonymous);

    // Check for admin role
    if !key_info.is_admin() {
        warn!(
            key_id = %key_info.id,
            role = %key_info.role,
            path = %path,
            "Non-admin access attempt to admin endpoint"
        );
        return Err(AuthError::InsufficientPermissions {
            required: ApiKeyRole::Admin,
            actual: key_info.role,
        });
    }

    // Check IP allowlist if configured
    if !config.ip_allowlist.is_empty() {
        // Note: In production, you'd extract the real client IP considering proxies
        // For now, we log a warning if allowlist is configured
        if config.log_access {
            info!(
                key_id = %key_info.id,
                path = %path,
                "Admin endpoint accessed"
            );
        }
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_api_key_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer sk-test-key-123"));

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("sk-test-key-123".to_string()));
    }

    #[test]
    fn test_extract_api_key_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("sk-test-key-456"));

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("sk-test-key-456".to_string()));
    }

    #[test]
    fn test_is_health_endpoint() {
        assert!(is_health_endpoint("/health"));
        assert!(is_health_endpoint("/healthz"));
        assert!(is_health_endpoint("/ready"));
        assert!(is_health_endpoint("/metrics"));
        assert!(!is_health_endpoint("/v1/chat/completions"));
        assert!(!is_health_endpoint("/admin/stats"));
    }

    #[test]
    fn test_is_admin_endpoint() {
        assert!(is_admin_endpoint("/admin/stats"));
        assert!(is_admin_endpoint("/admin/config"));
        assert!(!is_admin_endpoint("/v1/chat/completions"));
        assert!(!is_admin_endpoint("/health"));
    }

    #[test]
    fn test_api_key_info() {
        let entry = ApiKeyEntry {
            id: "key-1".to_string(),
            key: "sk-test".to_string(),
            role: ApiKeyRole::Admin,
            rate_limit_tier: crate::config::security::RateLimitTier::Admin,
            description: None,
            enabled: true,
        };

        let info = ApiKeyInfo::from_entry(&entry);
        assert!(info.is_admin());
        assert!(info.can_write());
        assert!(!info.anonymous);
    }

    #[test]
    fn test_anonymous_key_info() {
        let info = ApiKeyInfo::anonymous();
        assert!(!info.is_admin());
        assert!(!info.can_write());
        assert!(info.anonymous);
    }
}
