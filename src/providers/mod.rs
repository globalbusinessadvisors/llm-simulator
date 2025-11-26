//! Provider abstraction layer
//!
//! Provides a unified interface for different LLM API providers,
//! handling request/response translation and provider-specific behaviors.

mod openai;
mod anthropic;
mod google;

pub use openai::*;
pub use anthropic::*;
pub use google::*;

use async_trait::async_trait;
use crate::types::Provider;

/// Trait for provider implementations
#[async_trait]
pub trait ProviderHandler: Send + Sync {
    /// Get the provider type
    fn provider(&self) -> Provider;

    /// Check if this handler supports the given model
    fn supports_model(&self, model: &str) -> bool;

    /// Get the list of supported models
    fn supported_models(&self) -> Vec<String>;
}

/// Provider registry for routing requests
pub struct ProviderRegistry {
    handlers: Vec<Box<dyn ProviderHandler>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            handlers: vec![
                Box::new(OpenAIHandler::new()),
                Box::new(AnthropicHandler::new()),
                Box::new(GoogleHandler::new()),
            ],
        }
    }

    /// Find the provider for a given model
    pub fn find_provider(&self, model: &str) -> Option<Provider> {
        for handler in &self.handlers {
            if handler.supports_model(model) {
                return Some(handler.provider());
            }
        }
        None
    }

    /// Get all supported models
    pub fn all_models(&self) -> Vec<(String, Provider)> {
        let mut models = Vec::new();
        for handler in &self.handlers {
            for model in handler.supported_models() {
                models.push((model, handler.provider()));
            }
        }
        models
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Model ID normalization utilities
pub mod model_utils {
    /// Normalize model ID for consistent lookup
    pub fn normalize_model_id(model: &str) -> String {
        model.to_lowercase().trim().to_string()
    }

    /// Extract base model from versioned ID
    /// e.g., "gpt-4-0613" -> "gpt-4"
    pub fn base_model(model: &str) -> &str {
        // Common version suffixes
        let suffixes = ["-0613", "-0314", "-1106", "-0125", "-preview", "-latest"];

        for suffix in suffixes {
            if let Some(base) = model.strip_suffix(suffix) {
                return base;
            }
        }

        model
    }

    /// Check if model is a chat model (vs completion, embedding, etc.)
    pub fn is_chat_model(model: &str) -> bool {
        let chat_prefixes = [
            "gpt-4", "gpt-3.5-turbo",
            "claude-3", "claude-2",
            "gemini",
        ];

        let lower = model.to_lowercase();
        chat_prefixes.iter().any(|p| lower.starts_with(p))
    }

    /// Check if model is an embedding model
    pub fn is_embedding_model(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.contains("embedding") || lower.contains("embed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let registry = ProviderRegistry::new();

        assert_eq!(registry.find_provider("gpt-4"), Some(Provider::OpenAI));
        assert_eq!(registry.find_provider("claude-3-opus-20240229"), Some(Provider::Anthropic));
        assert_eq!(registry.find_provider("gemini-1.5-pro"), Some(Provider::Google));
    }

    #[test]
    fn test_model_normalization() {
        assert_eq!(model_utils::normalize_model_id("GPT-4"), "gpt-4");
        assert_eq!(model_utils::normalize_model_id("  gpt-4  "), "gpt-4");
    }

    #[test]
    fn test_base_model() {
        assert_eq!(model_utils::base_model("gpt-4-0613"), "gpt-4");
        assert_eq!(model_utils::base_model("gpt-4-turbo-preview"), "gpt-4-turbo");
        assert_eq!(model_utils::base_model("gpt-4"), "gpt-4");
    }

    #[test]
    fn test_model_type_detection() {
        assert!(model_utils::is_chat_model("gpt-4"));
        assert!(model_utils::is_chat_model("claude-3-opus-20240229"));
        assert!(!model_utils::is_chat_model("text-embedding-ada-002"));

        assert!(model_utils::is_embedding_model("text-embedding-ada-002"));
        assert!(model_utils::is_embedding_model("text-embedding-3-small"));
        assert!(!model_utils::is_embedding_model("gpt-4"));
    }
}
