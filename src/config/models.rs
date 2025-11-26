//! Model configuration definitions

use serde::{Deserialize, Serialize};
use crate::types::Provider;

/// Configuration for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    /// Model identifier
    pub id: String,
    /// Provider this model belongs to
    pub provider: Provider,
    /// Maximum context window size (tokens)
    pub context_length: usize,
    /// Maximum output tokens
    pub max_output_tokens: usize,
    /// Whether the model supports streaming
    pub supports_streaming: bool,
    /// Whether the model supports function/tool calling
    pub supports_functions: bool,
    /// Whether the model supports vision/images
    pub supports_vision: bool,
    /// Whether this is an embedding model
    pub is_embedding: bool,
    /// Embedding dimensions (for embedding models)
    pub embedding_dimensions: Option<usize>,
    /// Default response template
    pub default_response: Option<String>,
    /// Response generation config
    pub generation: GenerationConfig,
    /// Latency profile override
    pub latency_profile: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: false,
            is_embedding: false,
            embedding_dimensions: None,
            default_response: None,
            generation: GenerationConfig::default(),
            latency_profile: None,
        }
    }
}

impl ModelConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Model ID cannot be empty".to_string());
        }
        if self.context_length == 0 {
            return Err("context_length must be greater than 0".to_string());
        }
        // Embedding models don't produce output tokens, so allow 0 for them
        if self.max_output_tokens == 0 && !self.is_embedding {
            return Err("max_output_tokens must be greater than 0".to_string());
        }
        if self.is_embedding && self.embedding_dimensions.is_none() {
            return Err("Embedding models must specify embedding_dimensions".to_string());
        }
        Ok(())
    }

    // =========== OpenAI Models ===========

    pub fn gpt4() -> Self {
        Self {
            id: "gpt-4".to_string(),
            provider: Provider::OpenAI,
            context_length: 8_192,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: false,
            ..Default::default()
        }
    }

    pub fn gpt4_turbo() -> Self {
        Self {
            id: "gpt-4-turbo".to_string(),
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn gpt4o() -> Self {
        Self {
            id: "gpt-4o".to_string(),
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn gpt4o_mini() -> Self {
        Self {
            id: "gpt-4o-mini".to_string(),
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn gpt35_turbo() -> Self {
        Self {
            id: "gpt-3.5-turbo".to_string(),
            provider: Provider::OpenAI,
            context_length: 16_385,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: false,
            ..Default::default()
        }
    }

    // =========== Anthropic Models ===========

    pub fn claude_35_sonnet() -> Self {
        Self {
            id: "claude-3-5-sonnet-20241022".to_string(),
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn claude_3_opus() -> Self {
        Self {
            id: "claude-3-opus-20240229".to_string(),
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn claude_3_sonnet() -> Self {
        Self {
            id: "claude-3-sonnet-20240229".to_string(),
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn claude_3_haiku() -> Self {
        Self {
            id: "claude-3-haiku-20240307".to_string(),
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    // =========== Google Models ===========

    pub fn gemini_15_pro() -> Self {
        Self {
            id: "gemini-1.5-pro".to_string(),
            provider: Provider::Google,
            context_length: 2_000_000,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    pub fn gemini_15_flash() -> Self {
        Self {
            id: "gemini-1.5-flash".to_string(),
            provider: Provider::Google,
            context_length: 1_000_000,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            ..Default::default()
        }
    }

    // =========== Embedding Models ===========

    pub fn embedding_ada() -> Self {
        Self {
            id: "text-embedding-ada-002".to_string(),
            provider: Provider::OpenAI,
            context_length: 8191,
            max_output_tokens: 0,
            supports_streaming: false,
            supports_functions: false,
            supports_vision: false,
            is_embedding: true,
            embedding_dimensions: Some(1536),
            ..Default::default()
        }
    }

    pub fn embedding_3_small() -> Self {
        Self {
            id: "text-embedding-3-small".to_string(),
            provider: Provider::OpenAI,
            context_length: 8191,
            max_output_tokens: 0,
            supports_streaming: false,
            supports_functions: false,
            supports_vision: false,
            is_embedding: true,
            embedding_dimensions: Some(1536),
            ..Default::default()
        }
    }

    pub fn embedding_3_large() -> Self {
        Self {
            id: "text-embedding-3-large".to_string(),
            provider: Provider::OpenAI,
            context_length: 8191,
            max_output_tokens: 0,
            supports_streaming: false,
            supports_functions: false,
            supports_vision: false,
            is_embedding: true,
            embedding_dimensions: Some(3072),
            ..Default::default()
        }
    }
}

/// Configuration for response generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerationConfig {
    /// Minimum tokens to generate
    pub min_tokens: u32,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Token generation strategy
    pub strategy: GenerationStrategy,
    /// Response templates for simulation
    pub templates: Vec<String>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            min_tokens: 10,
            max_tokens: 500,
            strategy: GenerationStrategy::default(),
            templates: vec![
                "I'd be happy to help you with that.".to_string(),
                "Based on the information provided, here's my analysis:".to_string(),
                "Let me think about this step by step.".to_string(),
                "That's an interesting question. Here's what I can tell you:".to_string(),
            ],
        }
    }
}

/// Strategy for generating simulated responses
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStrategy {
    /// Use templates with variation
    #[default]
    Template,
    /// Generate lorem ipsum
    Lorem,
    /// Echo input with transformation
    Echo,
    /// Fixed response
    Fixed,
    /// Random tokens
    Random,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt4_config() {
        let config = ModelConfig::gpt4();
        assert_eq!(config.id, "gpt-4");
        assert_eq!(config.provider, Provider::OpenAI);
        assert!(config.supports_streaming);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_claude_config() {
        let config = ModelConfig::claude_35_sonnet();
        assert_eq!(config.provider, Provider::Anthropic);
        assert_eq!(config.context_length, 200_000);
        assert!(config.supports_vision);
    }

    #[test]
    fn test_embedding_config() {
        let config = ModelConfig::embedding_ada();
        assert!(config.is_embedding);
        assert_eq!(config.embedding_dimensions, Some(1536));
        assert!(!config.supports_streaming);
    }

    #[test]
    fn test_invalid_config() {
        let config = ModelConfig {
            id: "".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
