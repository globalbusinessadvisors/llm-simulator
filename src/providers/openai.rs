//! OpenAI provider implementation

use async_trait::async_trait;
use crate::types::Provider;
use super::ProviderHandler;

/// OpenAI API handler
pub struct OpenAIHandler {
    models: Vec<String>,
}

impl OpenAIHandler {
    pub fn new() -> Self {
        Self {
            models: vec![
                // GPT-4 family
                "gpt-4".to_string(),
                "gpt-4-0613".to_string(),
                "gpt-4-32k".to_string(),
                "gpt-4-32k-0613".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4-turbo-preview".to_string(),
                "gpt-4-1106-preview".to_string(),
                "gpt-4-0125-preview".to_string(),
                "gpt-4-vision-preview".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-2024-05-13".to_string(),
                "gpt-4o-2024-08-06".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4o-mini-2024-07-18".to_string(),
                // GPT-3.5 family
                "gpt-3.5-turbo".to_string(),
                "gpt-3.5-turbo-0613".to_string(),
                "gpt-3.5-turbo-16k".to_string(),
                "gpt-3.5-turbo-16k-0613".to_string(),
                "gpt-3.5-turbo-1106".to_string(),
                "gpt-3.5-turbo-0125".to_string(),
                // Embedding models
                "text-embedding-ada-002".to_string(),
                "text-embedding-3-small".to_string(),
                "text-embedding-3-large".to_string(),
                // O1 models
                "o1-preview".to_string(),
                "o1-mini".to_string(),
            ],
        }
    }

    /// Check if model ID matches OpenAI pattern
    pub fn matches_pattern(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.starts_with("gpt-")
            || lower.starts_with("text-embedding")
            || lower.starts_with("o1-")
            || lower.starts_with("davinci")
            || lower.starts_with("curie")
            || lower.starts_with("babbage")
            || lower.starts_with("ada")
    }
}

impl Default for OpenAIHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderHandler for OpenAIHandler {
    fn provider(&self) -> Provider {
        Provider::OpenAI
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m == model)
            || Self::matches_pattern(model)
    }

    fn supported_models(&self) -> Vec<String> {
        self.models.clone()
    }
}

/// OpenAI-specific request/response utilities
pub mod openai_utils {
    use crate::types::*;

    /// Convert internal messages to OpenAI format
    pub fn format_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|m| {
            let mut msg = serde_json::json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                    Role::Function => "function",
                },
            });

            match &m.content {
                MessageContent::Text(t) => {
                    msg["content"] = serde_json::Value::String(t.clone());
                }
                MessageContent::Parts(parts) => {
                    msg["content"] = serde_json::to_value(parts).unwrap_or_default();
                }
            }

            if let Some(name) = &m.name {
                msg["name"] = serde_json::Value::String(name.clone());
            }

            if let Some(tool_calls) = &m.tool_calls {
                msg["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
            }

            if let Some(tool_call_id) = &m.tool_call_id {
                msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
            }

            msg
        }).collect()
    }

    /// Generate OpenAI-style completion ID
    pub fn generate_completion_id() -> String {
        let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
        format!("chatcmpl-{}", &uuid[..24])
    }

    /// Generate OpenAI-style system fingerprint
    pub fn system_fingerprint() -> String {
        format!("fp_sim_{}", env!("CARGO_PKG_VERSION").replace('.', ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_handler() {
        let handler = OpenAIHandler::new();

        assert!(handler.supports_model("gpt-4"));
        assert!(handler.supports_model("gpt-4-turbo"));
        assert!(handler.supports_model("gpt-3.5-turbo"));
        assert!(handler.supports_model("text-embedding-ada-002"));
        assert!(!handler.supports_model("claude-3"));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(OpenAIHandler::matches_pattern("gpt-4-custom"));
        assert!(OpenAIHandler::matches_pattern("GPT-4"));
        assert!(OpenAIHandler::matches_pattern("text-embedding-custom"));
        assert!(!OpenAIHandler::matches_pattern("claude-3"));
    }

    #[test]
    fn test_completion_id_format() {
        let id = openai_utils::generate_completion_id();
        assert!(id.starts_with("chatcmpl-"));
        assert_eq!(id.len(), 33); // "chatcmpl-" + 24 chars
    }
}
