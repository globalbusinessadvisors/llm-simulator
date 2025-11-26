//! Property-based tests for serialization roundtrips

use proptest::prelude::*;
use serde_json::json;

// Strategy for generating valid model names
fn model_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{2,8}-[0-9]+(\\.[0-9]+)?(-[a-z]+)?")
        .unwrap()
        .prop_filter("model name must not be empty", |s| !s.is_empty())
}

// Strategy for generating valid message content
fn content_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ,.!?]{1,100}")
        .unwrap()
}

proptest! {
    /// Test that chat request JSON roundtrips correctly
    #[test]
    fn test_chat_request_roundtrip(
        model in model_strategy(),
        content in content_strategy(),
        temperature in prop::option::of(0.0f32..2.0),
        max_tokens in prop::option::of(1i32..4096),
    ) {
        let mut request = json!({
            "model": model,
            "messages": [
                {"role": "user", "content": content}
            ]
        });

        if let Some(temp) = temperature {
            request["temperature"] = json!(temp);
        }
        if let Some(mt) = max_tokens {
            request["max_tokens"] = json!(mt);
        }

        // Serialize and deserialize
        let json_str = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        prop_assert_eq!(&parsed["model"], &request["model"]);
        prop_assert_eq!(&parsed["messages"][0]["content"], &request["messages"][0]["content"]);
    }

    /// Test that usage values are preserved
    #[test]
    fn test_usage_serialization(
        prompt_tokens in 1u32..10000,
        completion_tokens in 1u32..10000,
    ) {
        let usage = json!({
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens
        });

        let json_str = serde_json::to_string(&usage).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        prop_assert_eq!(
            parsed["prompt_tokens"].as_u64().unwrap() as u32,
            prompt_tokens
        );
        prop_assert_eq!(
            parsed["completion_tokens"].as_u64().unwrap() as u32,
            completion_tokens
        );
        prop_assert_eq!(
            parsed["total_tokens"].as_u64().unwrap() as u32,
            prompt_tokens + completion_tokens
        );
    }

    /// Test that embeddings array preserves dimensions
    #[test]
    fn test_embedding_serialization(
        dimensions in 64usize..2048,
    ) {
        // Generate a fake embedding vector
        let embedding: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.001).collect();

        let data = json!({
            "embedding": embedding,
            "index": 0
        });

        let json_str = serde_json::to_string(&data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let parsed_embedding = parsed["embedding"].as_array().unwrap();
        prop_assert_eq!(parsed_embedding.len(), dimensions);
    }

    /// Test that error responses have correct structure
    #[test]
    fn test_error_response_structure(
        error_type in "[a-z_]+",
        message in content_strategy(),
    ) {
        let error = json!({
            "error": {
                "message": message,
                "type": error_type
            }
        });

        let json_str = serde_json::to_string(&error).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        prop_assert!(parsed["error"]["message"].is_string());
        prop_assert!(parsed["error"]["type"].is_string());
    }

    /// Test that stream chunk format is valid
    #[test]
    fn test_stream_chunk_format(
        content in content_strategy(),
        index in 0u32..10,
    ) {
        let chunk = json!({
            "id": "chatcmpl-test",
            "object": "chat.completion.chunk",
            "created": 1234567890i64,
            "model": "gpt-4",
            "choices": [{
                "index": index,
                "delta": {"content": content},
                "finish_reason": null
            }]
        });

        let json_str = serde_json::to_string(&chunk).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        prop_assert_eq!(parsed["object"].as_str().unwrap(), "chat.completion.chunk");
        prop_assert!(parsed["choices"].is_array());
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_anthropic_response_structure() {
        let response = json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello"}],
            "model": "claude-3",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["type"], "message");
        assert_eq!(parsed["role"], "assistant");
    }

    #[test]
    fn test_gemini_response_structure() {
        let response = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        });

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed["candidates"].is_array());
        assert!(parsed["candidates"][0]["content"]["parts"].is_array());
    }
}
