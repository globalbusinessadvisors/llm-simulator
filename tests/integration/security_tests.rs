//! Security integration tests

use super::common::*;
use llm_simulator::config::{
    SimulatorConfig,
    security::{ApiKeyConfig, ApiKeyEntry, ApiKeyRole, RateLimitTier},
};

fn config_with_auth() -> SimulatorConfig {
    let mut config = SimulatorConfig::default();
    config.security.api_keys = ApiKeyConfig {
        enabled: true,
        allow_anonymous_health: true,
        keys: vec![
            ApiKeyEntry {
                id: "user-key".to_string(),
                key: "sk-test-user-key".to_string(),
                role: ApiKeyRole::User,
                rate_limit_tier: RateLimitTier::Standard,
                description: Some("Test user key".to_string()),
                enabled: true,
            },
            ApiKeyEntry {
                id: "admin-key".to_string(),
                key: "sk-test-admin-key".to_string(),
                role: ApiKeyRole::Admin,
                rate_limit_tier: RateLimitTier::Admin,
                description: Some("Test admin key".to_string()),
                enabled: true,
            },
        ],
        ..Default::default()
    };
    config.security.admin.require_admin_key = true;
    config
}

#[tokio::test]
async fn test_health_without_auth() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // Health endpoint should work without auth
    let response = server.get("/health").await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_metrics_without_auth() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // Metrics endpoint should work without auth
    let response = server.get("/metrics").await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_api_endpoint_requires_auth() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // API endpoint without auth should fail
    let response = server
        .post("/v1/chat/completions", chat_request("gpt-4", "Test"))
        .await;

    assert_eq!(response.status().as_u16(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "error");
}

#[tokio::test]
async fn test_api_endpoint_with_valid_auth() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // API endpoint with valid auth should succeed
    let response = server
        .post_with_auth(
            "/v1/chat/completions",
            chat_request("gpt-4", "Test"),
            "sk-test-user-key",
        )
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_api_endpoint_with_invalid_auth() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // API endpoint with invalid key should fail
    let response = server
        .post_with_auth(
            "/v1/chat/completions",
            chat_request("gpt-4", "Test"),
            "sk-invalid-key",
        )
        .await;

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn test_admin_endpoint_requires_admin_role() {
    let mut config = config_with_auth();
    // Ensure admin endpoints require admin key
    config.security.admin.require_admin_key = true;

    let server = TestServer::spawn_with_config(config).await;

    // Admin endpoint with user key should fail (403 Forbidden)
    // When admin protection is enabled
    let response = server
        .client
        .get(server.url("/admin/stats"))
        .header("Authorization", "Bearer sk-test-user-key")
        .send()
        .await
        .unwrap();

    // Should be 403 (forbidden) because user role cannot access admin endpoints
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn test_admin_endpoint_with_admin_key() {
    let server = TestServer::spawn_with_config(config_with_auth()).await;

    // Admin endpoint with admin key should succeed
    let response = server
        .client
        .get(server.url("/admin/stats"))
        .header("Authorization", "Bearer sk-test-admin-key")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_security_headers_present() {
    let server = TestServer::spawn().await;

    let response = server.get("/health").await;

    // Check security headers
    let headers = response.headers();

    assert!(
        headers.get("x-content-type-options").is_some(),
        "X-Content-Type-Options header missing"
    );
    assert!(
        headers.get("x-frame-options").is_some(),
        "X-Frame-Options header missing"
    );
}

#[tokio::test]
async fn test_rate_limit_headers() {
    let server = TestServer::spawn().await;

    let response = server.get("/health").await;

    // Rate limit headers might be present
    // (depending on whether rate limiting is enabled)
    let _headers = response.headers();
    // Just checking the request doesn't fail
    assert_eq!(response.status().as_u16(), 200);
}
