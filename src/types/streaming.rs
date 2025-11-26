//! Streaming types for Server-Sent Events

use serde::{Deserialize, Serialize};
use super::{FinishReason, Role, Usage};

/// Streaming chat completion chunk (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionChunk {
    pub fn new(id: String, model: String, choices: Vec<ChunkChoice>) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model,
            choices,
            usage: None,
            system_fingerprint: Some(format!("fp_simulator_{}", env!("CARGO_PKG_VERSION").replace('.', ""))),
        }
    }

    /// Create a chunk with a content delta
    pub fn content_delta(id: String, model: String, content: String, index: u32) -> Self {
        Self::new(id, model, vec![ChunkChoice::content_delta(content, index)])
    }

    /// Create a chunk with finish reason
    pub fn finish(id: String, model: String, finish_reason: FinishReason, index: u32) -> Self {
        Self::new(id, model, vec![ChunkChoice::finish(finish_reason, index)])
    }

    /// Create a final chunk with usage
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Format as SSE data line
    pub fn to_sse_data(&self) -> String {
        format!("data: {}\n\n", serde_json::to_string(self).unwrap_or_default())
    }
}

/// A choice in a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

impl ChunkChoice {
    /// Create a choice with role delta (first chunk)
    pub fn role_delta(role: Role, index: u32) -> Self {
        Self {
            index,
            delta: ChunkDelta {
                role: Some(role),
                content: None,
                tool_calls: None,
                function_call: None,
            },
            finish_reason: None,
            logprobs: None,
        }
    }

    /// Create a choice with content delta
    pub fn content_delta(content: String, index: u32) -> Self {
        Self {
            index,
            delta: ChunkDelta {
                role: None,
                content: Some(content),
                tool_calls: None,
                function_call: None,
            },
            finish_reason: None,
            logprobs: None,
        }
    }

    /// Create a choice with finish reason
    pub fn finish(finish_reason: FinishReason, index: u32) -> Self {
        Self {
            index,
            delta: ChunkDelta::empty(),
            finish_reason: Some(finish_reason),
            logprobs: None,
        }
    }
}

/// Delta content in a streaming chunk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChunkToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<ChunkFunctionCall>,
}

impl ChunkDelta {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Tool call delta in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkToolCall {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ChunkFunctionCall>,
}

/// Function call delta in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkFunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Anthropic streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicStreamEvent {
    MessageStart {
        message: AnthropicStreamMessage,
    },
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlockType,
    },
    ContentBlockDelta {
        index: u32,
        delta: AnthropicDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: AnthropicMessageDelta,
        usage: AnthropicStreamUsage,
    },
    MessageStop,
    Ping,
    Error {
        error: AnthropicStreamError,
    },
}

/// Anthropic stream message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStreamMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicStreamUsage,
}

/// Anthropic content block type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlockType {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

/// Anthropic delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

/// Anthropic message delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

/// Anthropic stream usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnthropicStreamUsage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

/// Anthropic stream error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl AnthropicStreamEvent {
    /// Format as SSE data line
    pub fn to_sse_data(&self) -> String {
        let event_type = match self {
            Self::MessageStart { .. } => "message_start",
            Self::ContentBlockStart { .. } => "content_block_start",
            Self::ContentBlockDelta { .. } => "content_block_delta",
            Self::ContentBlockStop { .. } => "content_block_stop",
            Self::MessageDelta { .. } => "message_delta",
            Self::MessageStop => "message_stop",
            Self::Ping => "ping",
            Self::Error { .. } => "error",
        };
        format!(
            "event: {}\ndata: {}\n\n",
            event_type,
            serde_json::to_string(self).unwrap_or_default()
        )
    }
}

/// Stream state for tracking generation progress
#[derive(Debug, Clone, Default)]
pub struct StreamState {
    pub tokens_generated: u32,
    pub content_so_far: String,
    pub finished: bool,
    pub finish_reason: Option<FinishReason>,
}

impl StreamState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, content: &str) {
        self.content_so_far.push_str(content);
        self.tokens_generated += 1;
    }

    pub fn finish(&mut self, reason: FinishReason) {
        self.finished = true;
        self.finish_reason = Some(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = ChatCompletionChunk::content_delta(
            "chatcmpl-123".to_string(),
            "gpt-4".to_string(),
            "Hello".to_string(),
            0,
        );

        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_sse_format() {
        let chunk = ChatCompletionChunk::content_delta(
            "test".to_string(),
            "gpt-4".to_string(),
            "Hi".to_string(),
            0,
        );

        let sse = chunk.to_sse_data();
        assert!(sse.starts_with("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn test_stream_state() {
        let mut state = StreamState::new();
        state.append("Hello");
        state.append(" World");

        assert_eq!(state.content_so_far, "Hello World");
        assert_eq!(state.tokens_generated, 2);
        assert!(!state.finished);

        state.finish(FinishReason::Stop);
        assert!(state.finished);
    }
}
