//! Google provider implementation

use async_trait::async_trait;
use crate::types::Provider;
use super::ProviderHandler;

/// Google/Gemini API handler
pub struct GoogleHandler {
    models: Vec<String>,
}

impl GoogleHandler {
    pub fn new() -> Self {
        Self {
            models: vec![
                // Gemini 1.5 family
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-pro-latest".to_string(),
                "gemini-1.5-pro-001".to_string(),
                "gemini-1.5-pro-002".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-flash-latest".to_string(),
                "gemini-1.5-flash-001".to_string(),
                "gemini-1.5-flash-002".to_string(),
                "gemini-1.5-flash-8b".to_string(),
                // Gemini 1.0 family
                "gemini-1.0-pro".to_string(),
                "gemini-1.0-pro-001".to_string(),
                "gemini-1.0-pro-latest".to_string(),
                "gemini-1.0-pro-vision-latest".to_string(),
                // Gemini Pro (aliases)
                "gemini-pro".to_string(),
                "gemini-pro-vision".to_string(),
                // Embedding models
                "text-embedding-004".to_string(),
                "embedding-001".to_string(),
            ],
        }
    }

    /// Check if model ID matches Google/Gemini pattern
    pub fn matches_pattern(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.starts_with("gemini")
            || lower.starts_with("text-embedding-004")
            || lower.starts_with("embedding-")
            || lower.contains("palm")
    }
}

impl Default for GoogleHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderHandler for GoogleHandler {
    fn provider(&self) -> Provider {
        Provider::Google
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m == model)
            || Self::matches_pattern(model)
    }

    fn supported_models(&self) -> Vec<String> {
        self.models.clone()
    }
}

/// Google/Gemini-specific utilities
pub mod google_utils {
    use crate::types::*;

    /// Convert internal messages to Gemini format
    pub fn convert_messages(messages: &[Message]) -> Vec<GeminiContent> {
        let mut contents = Vec::new();

        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model",
                Role::System => "user", // Gemini treats system as user with context
                _ => "user",
            };

            contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart {
                    text: Some(msg.text()),
                    inline_data: None,
                }],
            });
        }

        contents
    }

    /// Convert OpenAI-style request to Gemini format
    pub fn from_openai_request(request: &ChatCompletionRequest) -> GeminiRequest {
        let contents = convert_messages(&request.messages);

        let generation_config = Some(GeminiGenerationConfig {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_output_tokens: Some(request.effective_max_tokens()),
            stop_sequences: request.stop.as_ref().map(|s| s.to_vec()),
        });

        GeminiRequest {
            contents,
            generation_config,
            safety_settings: None,
        }
    }

    /// Convert Gemini response to OpenAI format
    pub fn to_openai_response(
        response: GeminiResponse,
        request_model: &str,
    ) -> ChatCompletionResponse {
        let content = response.candidates
            .first()
            .map(|c| {
                c.content.parts.iter()
                    .map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let (input_tokens, output_tokens) = response.usage_metadata
            .map(|u| (u.prompt_token_count, u.candidates_token_count))
            .unwrap_or((0, 0));

        let id = format!("chatcmpl-gemini-{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..12].to_string());

        ChatCompletionResponse::simple(
            id,
            request_model.to_string(),
            content,
            Usage::new(input_tokens, output_tokens),
        )
    }

    /// Map Gemini finish reason to OpenAI format
    pub fn map_finish_reason(reason: Option<&str>) -> FinishReason {
        match reason {
            Some("STOP") => FinishReason::Stop,
            Some("MAX_TOKENS") => FinishReason::Length,
            Some("SAFETY") => FinishReason::ContentFilter,
            Some("RECITATION") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_handler() {
        let handler = GoogleHandler::new();

        assert!(handler.supports_model("gemini-1.5-pro"));
        assert!(handler.supports_model("gemini-1.5-flash"));
        assert!(handler.supports_model("gemini-pro"));
        assert!(!handler.supports_model("gpt-4"));
        assert!(!handler.supports_model("claude-3"));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(GoogleHandler::matches_pattern("gemini-custom"));
        assert!(GoogleHandler::matches_pattern("Gemini-1.5-pro"));
        assert!(!GoogleHandler::matches_pattern("gpt-4"));
    }

    #[test]
    fn test_message_conversion() {
        use crate::types::Message;

        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi!"),
        ];

        let converted = google_utils::convert_messages(&messages);

        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "model");
    }

    #[test]
    fn test_finish_reason_mapping() {
        use crate::types::FinishReason;
        assert_eq!(google_utils::map_finish_reason(Some("STOP")), FinishReason::Stop);
        assert_eq!(google_utils::map_finish_reason(Some("MAX_TOKENS")), FinishReason::Length);
        assert_eq!(google_utils::map_finish_reason(Some("SAFETY")), FinishReason::ContentFilter);
    }
}
