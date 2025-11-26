//! Security Headers Middleware
//!
//! Adds security headers to all HTTP responses to protect against
//! common web vulnerabilities:
//! - XSS protection
//! - Clickjacking prevention
//! - MIME sniffing prevention
//! - Content Security Policy
//! - HTTP Strict Transport Security

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};

use crate::config::security::SecurityHeadersConfig;

/// Static header values for performance
struct StaticHeaders {
    x_content_type_options: HeaderValue,
    x_frame_options: HeaderValue,
    x_xss_protection: HeaderValue,
    referrer_policy: HeaderValue,
    cache_control: HeaderValue,
}

impl StaticHeaders {
    fn new() -> Self {
        Self {
            x_content_type_options: HeaderValue::from_static("nosniff"),
            x_frame_options: HeaderValue::from_static("DENY"),
            x_xss_protection: HeaderValue::from_static("1; mode=block"),
            referrer_policy: HeaderValue::from_static("strict-origin-when-cross-origin"),
            cache_control: HeaderValue::from_static("no-store, max-age=0"),
        }
    }
}

static STATIC_HEADERS: once_cell::sync::Lazy<StaticHeaders> =
    once_cell::sync::Lazy::new(StaticHeaders::new);

/// Security headers middleware
pub async fn security_headers_middleware(
    State(config): State<Arc<SecurityHeadersConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    if !config.enabled {
        return response;
    }

    let headers = response.headers_mut();

    // Prevent MIME sniffing
    headers.insert(
        "x-content-type-options",
        STATIC_HEADERS.x_content_type_options.clone(),
    );

    // Prevent clickjacking
    if let Ok(value) = HeaderValue::from_str(&config.frame_options) {
        headers.insert("x-frame-options", value);
    } else {
        headers.insert(
            "x-frame-options",
            STATIC_HEADERS.x_frame_options.clone(),
        );
    }

    // XSS protection (legacy browsers)
    headers.insert(
        "x-xss-protection",
        STATIC_HEADERS.x_xss_protection.clone(),
    );

    // Referrer policy
    if let Ok(value) = HeaderValue::from_str(&config.referrer_policy) {
        headers.insert("referrer-policy", value);
    } else {
        headers.insert(
            "referrer-policy",
            STATIC_HEADERS.referrer_policy.clone(),
        );
    }

    // Content Security Policy
    if let Some(csp) = &config.content_security_policy {
        if let Ok(value) = HeaderValue::from_str(csp) {
            headers.insert("content-security-policy", value);
        }
    }

    // Permissions Policy
    if let Some(pp) = &config.permissions_policy {
        if let Ok(value) = HeaderValue::from_str(pp) {
            headers.insert("permissions-policy", value);
        }
    }

    // HSTS (only when enabled and typically behind TLS)
    if config.hsts_enabled {
        let mut hsts_value = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts_value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            hsts_value.push_str("; preload");
        }
        if let Ok(value) = HeaderValue::from_str(&hsts_value) {
            headers.insert("strict-transport-security", value);
        }
    }

    // Cache control for API responses (prevent caching of sensitive data)
    if !headers.contains_key("cache-control") {
        headers.insert("cache-control", STATIC_HEADERS.cache_control.clone());
    }

    response
}

/// Create a default security headers config for production
pub fn production_security_headers() -> SecurityHeadersConfig {
    SecurityHeadersConfig {
        enabled: true,
        hsts_enabled: true,
        hsts_max_age: 31536000, // 1 year
        hsts_include_subdomains: true,
        hsts_preload: false,
        content_security_policy: Some("default-src 'none'; frame-ancestors 'none'".to_string()),
        frame_options: "DENY".to_string(),
        referrer_policy: "strict-origin-when-cross-origin".to_string(),
        permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_headers() {
        let headers = &*STATIC_HEADERS;
        assert_eq!(headers.x_content_type_options.to_str().unwrap(), "nosniff");
        assert_eq!(headers.x_frame_options.to_str().unwrap(), "DENY");
    }

    #[test]
    fn test_production_security_headers() {
        let config = production_security_headers();
        assert!(config.enabled);
        assert!(config.hsts_enabled);
        assert!(config.hsts_include_subdomains);
        assert!(config.content_security_policy.is_some());
    }
}
