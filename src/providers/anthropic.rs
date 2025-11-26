//! Anthropic provider implementation

use async_trait::async_trait;
use crate::types::Provider;
use super::ProviderHandler;

/// Anthropic API handler
pub struct AnthropicHandler {
    models: Vec<String>,
}

impl AnthropicHandler {
    pub fn new() -> Self {
        Self {
            models: vec![
                // Claude 3.5 family
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-sonnet-20240620".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                // Claude 3 family
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
                // Claude 2 family
                "claude-2.1".to_string(),
                "claude-2.0".to_string(),
                // Claude Instant
                "claude-instant-1.2".to_string(),
            ],
        }
    }

    /// Check if model ID matches Anthropic pattern
    pub fn matches_pattern(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.starts_with("claude")
    }
}

impl Default for AnthropicHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderHandler for AnthropicHandler {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m == model)
            || Self::matches_pattern(model)
    }

    fn supported_models(&self) -> Vec<String> {
        self.models.clone()
    }
}

/// Anthropic-specific utilities
pub mod anthropic_utils {
    use crate::types::*;

    /// Convert internal messages to Anthropic format
    pub fn convert_messages(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_prompt = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system_prompt = Some(msg.text());
                }
                Role::User => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Text(msg.text()),
                    });
                }
                Role::Assistant => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Text(msg.text()),
                    });
                }
                _ => {
                    // Tool messages need special handling
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Text(msg.text()),
                    });
                }
            }
        }

        (system_prompt, anthropic_messages)
    }

    /// Generate Anthropic-style message ID
    pub fn generate_message_id() -> String {
        let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
        format!("msg_{}", &uuid[..24])
    }

    /// Convert OpenAI-style request to Anthropic format
    pub fn from_openai_request(request: &ChatCompletionRequest) -> AnthropicMessagesRequest {
        let (system, messages) = convert_messages(&request.messages);

        AnthropicMessagesRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.effective_max_tokens(),
            system,
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            stream: request.stream,
            stop_sequences: request.stop.as_ref().map(|s| s.to_vec()),
            tools: None,
        }
    }

    /// Convert Anthropic response to OpenAI format
    pub fn to_openai_response(
        response: AnthropicMessagesResponse,
        request_model: &str,
    ) -> ChatCompletionResponse {
        let content = response.content.iter()
            .filter_map(|block| {
                match block {
                    AnthropicContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join("");

        ChatCompletionResponse::simple(
            response.id,
            request_model.to_string(),
            content,
            Usage::new(response.usage.input_tokens, response.usage.output_tokens),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_handler() {
        let handler = AnthropicHandler::new();

        assert!(handler.supports_model("claude-3-5-sonnet-20241022"));
        assert!(handler.supports_model("claude-3-opus-20240229"));
        assert!(handler.supports_model("claude-2.1"));
        assert!(!handler.supports_model("gpt-4"));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(AnthropicHandler::matches_pattern("claude-3-custom"));
        assert!(AnthropicHandler::matches_pattern("Claude-3"));
        assert!(!AnthropicHandler::matches_pattern("gpt-4"));
    }

    #[test]
    fn test_message_id_format() {
        let id = anthropic_utils::generate_message_id();
        assert!(id.starts_with("msg_"));
    }

    #[test]
    fn test_message_conversion() {
        use crate::types::Message;

        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let (system, converted) = anthropic_utils::convert_messages(&messages);

        assert_eq!(system, Some("You are a helpful assistant".to_string()));
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "assistant");
    }
}
