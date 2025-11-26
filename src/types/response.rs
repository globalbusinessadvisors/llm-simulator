//! Response types for LLM APIs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::{FinishReason, Role, Usage};

/// Chat completion response (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionResponse {
    pub fn new(id: String, model: String, choices: Vec<ChatCompletionChoice>, usage: Usage) -> Self {
        Self {
            id,
            object: "chat.completion".to_string(),
            created: Utc::now().timestamp(),
            model,
            choices,
            usage: Some(usage),
            system_fingerprint: Some(format!("fp_simulator_{}", env!("CARGO_PKG_VERSION").replace('.', ""))),
        }
    }

    /// Create a simple response with a single text choice
    pub fn simple(id: String, model: String, content: String, usage: Usage) -> Self {
        let choice = ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: Role::Assistant,
                content: Some(content),
                tool_calls: None,
                function_call: None,
            },
            finish_reason: Some(FinishReason::Stop),
            logprobs: None,
        };
        Self::new(id, model, vec![choice], usage)
    }
}

/// A choice in the chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,
}

/// Message in a chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ResponseToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<ResponseFunctionCall>,
}

/// Tool call in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ResponseFunctionCall,
}

/// Function call in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Log probabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogProbs {
    pub content: Option<Vec<TokenLogProb>>,
}

/// Token log probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogProb {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<TopLogProb>,
}

/// Top log probability entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLogProb {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
}

/// Embeddings response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub object: String,
    pub data: Vec<EmbeddingObject>,
    pub model: String,
    pub usage: EmbeddingUsage,
}

impl EmbeddingsResponse {
    pub fn new(model: String, embeddings: Vec<Vec<f32>>, total_tokens: u32) -> Self {
        let data = embeddings
            .into_iter()
            .enumerate()
            .map(|(i, embedding)| EmbeddingObject {
                object: "embedding".to_string(),
                index: i as u32,
                embedding,
            })
            .collect();

        Self {
            object: "list".to_string(),
            data,
            model,
            usage: EmbeddingUsage {
                prompt_tokens: total_tokens,
                total_tokens,
            },
        }
    }
}

/// Individual embedding object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingObject {
    pub object: String,
    pub index: u32,
    pub embedding: Vec<f32>,
}

/// Embedding usage info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// Models list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelObject>,
}

impl ModelsResponse {
    pub fn new(models: Vec<ModelObject>) -> Self {
        Self {
            object: "list".to_string(),
            data: models,
        }
    }
}

/// Individual model object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

impl ModelObject {
    pub fn new(id: impl Into<String>, owned_by: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            object: "model".to_string(),
            created: Utc::now().timestamp(),
            owned_by: owned_by.into(),
        }
    }
}

/// Anthropic Messages API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

impl AnthropicMessagesResponse {
    pub fn new(
        id: String,
        model: String,
        content: String,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Self {
        Self {
            id,
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![AnthropicContentBlock::Text { text: content }],
            model,
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: AnthropicUsage {
                input_tokens,
                output_tokens,
            },
        }
    }
}

/// Anthropic content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Anthropic usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Google Gemini response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

impl GeminiResponse {
    pub fn new(content: String, input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            candidates: vec![GeminiCandidate {
                content: GeminiResponseContent {
                    role: "model".to_string(),
                    parts: vec![GeminiResponsePart { text: content }],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: None,
            }],
            usage_metadata: Some(GeminiUsageMetadata {
                prompt_token_count: input_tokens,
                candidates_token_count: output_tokens,
                total_token_count: input_tokens + output_tokens,
            }),
        }
    }
}

/// Gemini candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiResponseContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<serde_json::Value>>,
}

/// Gemini response content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiResponseContent {
    pub role: String,
    pub parts: Vec<GeminiResponsePart>,
}

/// Gemini response part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiResponsePart {
    pub text: String,
}

/// Gemini usage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: DateTime<Utc>,
}

impl HealthResponse {
    pub fn healthy(version: &str, uptime: std::time::Duration) -> Self {
        Self {
            status: "healthy".to_string(),
            version: version.to_string(),
            uptime_seconds: uptime.as_secs(),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_response_creation() {
        let response = ChatCompletionResponse::simple(
            "chatcmpl-123".to_string(),
            "gpt-4".to_string(),
            "Hello!".to_string(),
            Usage::new(10, 5),
        );

        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Hello!".to_string()));
    }

    #[test]
    fn test_embeddings_response() {
        let response = EmbeddingsResponse::new(
            "text-embedding-ada-002".to_string(),
            vec![vec![0.1, 0.2, 0.3]],
            10,
        );

        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding.len(), 3);
    }

    #[test]
    fn test_response_serialization() {
        let response = ChatCompletionResponse::simple(
            "test".to_string(),
            "gpt-4".to_string(),
            "Hi".to_string(),
            Usage::new(1, 1),
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("chat.completion"));
    }
}
