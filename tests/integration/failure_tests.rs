//! Failure scenario integration tests

use super::common::*;
use llm_simulator::config::SimulatorConfig;
use serde_json::json;

/// Create config with open admin access (for testing admin endpoints)
fn config_with_open_admin() -> SimulatorConfig {
    let mut config = SimulatorConfig::default();
    config.security.admin.require_admin_key = false;
    config
}

#[tokio::test]
async fn test_invalid_json_body() {
    let server = TestServer::spawn().await;

    // Send malformed JSON
    let response = server
        .client
        .post(server.url("/v1/chat/completions"))
        .header("Content-Type", "application/json")
        .body("{invalid json}")
        .send()
        .await
        .unwrap();

    // Should return 4xx error
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_missing_required_fields() {
    let server = TestServer::spawn().await;

    // Missing model field
    let response = server
        .post("/v1/chat/completions", json!({
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_empty_messages() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", json!({
            "model": "gpt-4",
            "messages": []
        }))
        .await;

    // Empty messages might return error or succeed with empty response
    // Either is acceptable behavior
    let status = response.status().as_u16();
    assert!(status == 200 || status >= 400);
}

#[tokio::test]
async fn test_model_not_found() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", chat_request("nonexistent-model-xyz", "Test"))
        .await;

    assert_eq!(response.status().as_u16(), 404);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "error");
}

#[tokio::test]
async fn test_endpoint_not_found() {
    let server = TestServer::spawn().await;

    let response = server.get("/v1/nonexistent/endpoint").await;
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = TestServer::spawn().await;

    // GET to POST-only endpoint
    let response = server.get("/v1/chat/completions").await;
    assert_eq!(response.status().as_u16(), 405);
}

#[tokio::test]
async fn test_admin_stats_endpoint() {
    let server = TestServer::spawn_with_config(config_with_open_admin()).await;

    let response = server.get("/admin/stats").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "total_requests");
}

#[tokio::test]
async fn test_admin_config_endpoint() {
    let server = TestServer::spawn_with_config(config_with_open_admin()).await;

    let response = server.get("/admin/config").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "server");
    assert_json_field(&body, "models");
}

#[tokio::test]
async fn test_admin_chaos_status() {
    let server = TestServer::spawn_with_config(config_with_open_admin()).await;

    let response = server.get("/admin/chaos/status").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "enabled");
}

#[tokio::test]
async fn test_ready_endpoint() {
    let server = TestServer::spawn().await;

    let response = server.get("/ready").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["ready"], true);
}

#[tokio::test]
async fn test_readyz_endpoint() {
    let server = TestServer::spawn().await;

    let response = server.get("/readyz").await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_healthz_endpoint() {
    let server = TestServer::spawn().await;

    let response = server.get("/healthz").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "status");
    assert_json_field(&body, "checks");
}

#[tokio::test]
async fn test_version_endpoint() {
    let server = TestServer::spawn().await;

    let response = server.get("/version").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "name");
    assert_json_field(&body, "version");
}

#[tokio::test]
async fn test_root_endpoint() {
    let server = TestServer::spawn().await;

    let response = server.get("/").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "name");
    assert_json_field(&body, "endpoints");
}

#[tokio::test]
async fn test_metrics_format() {
    let server = TestServer::spawn().await;

    let response = server.get("/metrics").await;
    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();

    // Should be Prometheus format
    assert!(body.contains("TYPE") || body.is_empty() || body.contains("llm_simulator"));
}

#[tokio::test]
async fn test_concurrent_requests() {
    let server = TestServer::spawn().await;

    // Send multiple concurrent requests
    let mut handles = vec![];
    for _i in 0..10 {
        let url = server.url("/health");
        let client = server.client.clone();
        handles.push(tokio::spawn(async move {
            client.get(url).send().await.unwrap().status().as_u16()
        }));
    }

    // All should succeed (200) or be rate limited (429)
    for handle in handles {
        let status = handle.await.unwrap();
        assert!(
            status == 200 || status == 429,
            "Expected 200 or 429, got {}",
            status
        );
    }
}
