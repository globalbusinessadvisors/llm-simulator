//! Anthropic API endpoint integration tests

use super::common::*;
use serde_json::json;

#[tokio::test]
async fn test_anthropic_messages_basic() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/messages", anthropic_request("claude-3-5-sonnet-20241022", "Hello!", 100))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "id");
    assert_json_field(&body, "type");
    assert_json_field(&body, "content");
    assert_json_field(&body, "usage");

    assert_eq!(body["type"], "message");
    assert_eq!(body["role"], "assistant");
}

#[tokio::test]
async fn test_anthropic_messages_with_system() {
    let server = TestServer::spawn().await;

    let request = json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 100,
        "system": "You are a helpful assistant.",
        "messages": [
            {"role": "user", "content": "Hello!"}
        ]
    });

    let response = server.post("/v1/messages", request).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["content"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_anthropic_messages_claude_opus() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/messages", anthropic_request("claude-3-opus-20240229", "Test", 50))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["model"], "claude-3-opus-20240229");
}

#[tokio::test]
async fn test_anthropic_messages_claude_haiku() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/messages", anthropic_request("claude-3-haiku-20240307", "Quick test", 50))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["model"], "claude-3-haiku-20240307");
}

#[tokio::test]
async fn test_anthropic_messages_without_version_prefix() {
    let server = TestServer::spawn().await;

    // /messages endpoint without /v1 prefix
    let response = server
        .post("/messages", anthropic_request("claude-3-5-sonnet-20241022", "Hello!", 100))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_anthropic_messages_multiple_turns() {
    let server = TestServer::spawn().await;

    let request = json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 100,
        "messages": [
            {"role": "user", "content": "My name is Alice."},
            {"role": "assistant", "content": "Nice to meet you, Alice!"},
            {"role": "user", "content": "What's my name?"}
        ]
    });

    let response = server.post("/v1/messages", request).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["content"][0]["text"].as_str().is_some());
}

#[tokio::test]
async fn test_anthropic_usage_fields() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/messages", anthropic_request("claude-3-5-sonnet-20241022", "Test usage", 100))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let usage = &body["usage"];

    assert!(usage["input_tokens"].as_i64().unwrap() > 0);
    assert!(usage["output_tokens"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_anthropic_stop_reason() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/messages", anthropic_request("claude-3-5-sonnet-20241022", "Hello", 100))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["stop_reason"], "end_turn");
}
