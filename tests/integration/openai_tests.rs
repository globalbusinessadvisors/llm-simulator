//! OpenAI API endpoint integration tests

use super::common::*;
use serde_json::json;

#[tokio::test]
async fn test_chat_completions_basic() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", chat_request("gpt-4", "Hello, world!"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "id");
    assert_json_field(&body, "object");
    assert_json_field(&body, "choices");
    assert_json_field(&body, "usage");

    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_chat_completions_with_temperature() {
    let server = TestServer::spawn().await;

    let request = chat_request_with_options(
        "gpt-4",
        vec![json!({"role": "user", "content": "Test"})],
        Some(100),
        Some(0.7),
        false,
    );

    let response = server.post("/v1/chat/completions", request).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "choices");
}

#[tokio::test]
async fn test_chat_completions_multiple_messages() {
    let server = TestServer::spawn().await;

    let messages = vec![
        json!({"role": "system", "content": "You are a helpful assistant."}),
        json!({"role": "user", "content": "Hello!"}),
        json!({"role": "assistant", "content": "Hi there!"}),
        json!({"role": "user", "content": "How are you?"}),
    ];

    let request = chat_request_with_options("gpt-4", messages, Some(50), None, false);

    let response = server.post("/v1/chat/completions", request).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["choices"][0]["message"]["content"].as_str().is_some());
}

#[tokio::test]
async fn test_chat_completions_gpt35_turbo() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", chat_request("gpt-3.5-turbo", "Test message"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["model"], "gpt-3.5-turbo");
}

#[tokio::test]
async fn test_chat_completions_gpt4_turbo() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", chat_request("gpt-4-turbo", "Test"))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_chat_completions_invalid_model() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/chat/completions", chat_request("nonexistent-model", "Test"))
        .await;

    assert_eq!(response.status().as_u16(), 404);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "error");
}

#[tokio::test]
async fn test_embeddings_basic() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/embeddings", embeddings_request("text-embedding-ada-002", "Hello world"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "data");
    assert_json_field(&body, "usage");
    assert_eq!(body["object"], "list");

    let embeddings = body["data"].as_array().unwrap();
    assert!(embeddings.len() > 0);
    assert!(embeddings[0]["embedding"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_embeddings_3_small() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/embeddings", embeddings_request("text-embedding-3-small", "Test embedding"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["model"], "text-embedding-3-small");
}

#[tokio::test]
async fn test_list_models() {
    let server = TestServer::spawn().await;

    let response = server.get("/v1/models").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["object"], "list");

    let models = body["data"].as_array().unwrap();
    assert!(models.len() > 0);

    // Check that expected models exist
    let model_ids: Vec<&str> = models
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();

    assert!(model_ids.contains(&"gpt-4"));
    assert!(model_ids.contains(&"gpt-3.5-turbo"));
}

#[tokio::test]
async fn test_get_model() {
    let server = TestServer::spawn().await;

    let response = server.get("/v1/models/gpt-4").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"], "gpt-4");
    assert_eq!(body["object"], "model");
}

#[tokio::test]
async fn test_get_model_not_found() {
    let server = TestServer::spawn().await;

    let response = server.get("/v1/models/nonexistent").await;
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_legacy_completions_endpoint() {
    let server = TestServer::spawn().await;

    // Legacy /v1/completions should work like chat
    let response = server
        .post("/v1/completions", chat_request("gpt-4", "Test"))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}
