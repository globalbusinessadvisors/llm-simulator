//! Google/Gemini API endpoint integration tests

use super::common::*;
use serde_json::json;

#[tokio::test]
async fn test_gemini_generate_content_basic() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/models/gemini-1.5-pro/generateContent", gemini_request("Hello!"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_json_field(&body, "candidates");
    assert_json_field(&body, "usage_metadata");

    let candidates = body["candidates"].as_array().unwrap();
    assert!(candidates.len() > 0);
}

#[tokio::test]
async fn test_gemini_generate_content_flash() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/models/gemini-1.5-flash/generateContent", gemini_request("Quick test"))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_gemini_generate_content_with_config() {
    let server = TestServer::spawn().await;

    let request = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello!"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 100,
            "topP": 0.9
        }
    });

    let response = server
        .post("/v1/models/gemini-1.5-pro/generateContent", request)
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_gemini_beta_endpoint() {
    let server = TestServer::spawn().await;

    // v1beta endpoint should also work
    let response = server
        .post("/v1beta/models/gemini-1.5-pro/generateContent", gemini_request("Test"))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_gemini_usage_metadata() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/models/gemini-1.5-pro/generateContent", gemini_request("Test usage"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let usage = &body["usage_metadata"];

    assert!(usage["prompt_token_count"].as_i64().unwrap() > 0);
    assert!(usage["candidates_token_count"].as_i64().unwrap() > 0);
    assert!(usage["total_token_count"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_gemini_candidate_content() {
    let server = TestServer::spawn().await;

    let response = server
        .post("/v1/models/gemini-1.5-pro/generateContent", gemini_request("Hello"))
        .await;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let candidate = &body["candidates"][0];

    assert_eq!(candidate["content"]["role"], "model");
    assert!(candidate["content"]["parts"][0]["text"].as_str().is_some());
    assert_eq!(candidate["finish_reason"], "STOP");
}

#[tokio::test]
async fn test_gemini_multi_turn_conversation() {
    let server = TestServer::spawn().await;

    let request = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "My name is Bob."}]
            },
            {
                "role": "model",
                "parts": [{"text": "Hello Bob!"}]
            },
            {
                "role": "user",
                "parts": [{"text": "What is my name?"}]
            }
        ]
    });

    let response = server
        .post("/v1/models/gemini-1.5-pro/generateContent", request)
        .await;

    assert_eq!(response.status().as_u16(), 200);
}
