//! API types for LLM-Simulator
//!
//! This module defines request and response types compatible with
//! OpenAI, Anthropic, Google, and Azure APIs.

mod messages;
mod request;
mod response;
mod streaming;

pub use messages::*;
pub use request::*;
pub use response::*;
pub use streaming::*;

use serde::{Deserialize, Serialize};

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    OpenAI,
    Anthropic,
    Google,
    Azure,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Google => write!(f, "google"),
            Self::Azure => write!(f, "azure"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "anthropic" => Ok(Self::Anthropic),
            "google" => Ok(Self::Google),
            "azure" => Ok(Self::Azure),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Finish reason for completions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    FunctionCall,
}

impl Default for FinishReason {
    fn default() -> Self {
        Self::Stop
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: Provider,
    pub context_length: usize,
    pub max_output_tokens: usize,
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub supports_vision: bool,
    #[serde(default)]
    pub pricing: ModelPricing,
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self {
            id: "gpt-4".to_string(),
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            pricing: ModelPricing::default(),
        }
    }
}

/// Model pricing information (per 1K tokens)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
}

/// Role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

impl Default for Role {
    fn default() -> Self {
        Self::User
    }
}
