//! SDK Configuration
//!
//! Configuration options for the LLM-Simulator client.

use std::time::Duration;

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the LLM-Simulator instance
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Request timeout
    pub timeout: Duration,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Base delay for exponential backoff
    pub retry_delay: Duration,

    /// Maximum delay for exponential backoff
    pub max_retry_delay: Duration,

    /// User agent string
    pub user_agent: String,

    /// Default model to use
    pub default_model: Option<String>,

    /// Default temperature
    pub default_temperature: Option<f32>,

    /// Default max tokens
    pub default_max_tokens: Option<u32>,

    /// Enable request/response logging
    pub enable_logging: bool,

    /// Provider format (openai, anthropic, google)
    pub provider: Provider,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(10),
            user_agent: format!("llm-simulator-sdk/{}", crate::VERSION),
            default_model: None,
            default_temperature: None,
            default_max_tokens: None,
            enable_logging: false,
            provider: Provider::OpenAI,
        }
    }
}

impl ClientConfig {
    /// Create a new client configuration with the given base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the retry delay
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Set the default temperature
    pub fn default_temperature(mut self, temp: f32) -> Self {
        self.default_temperature = Some(temp);
        self
    }

    /// Set the default max tokens
    pub fn default_max_tokens(mut self, tokens: u32) -> Self {
        self.default_max_tokens = Some(tokens);
        self
    }

    /// Enable request/response logging
    pub fn enable_logging(mut self) -> Self {
        self.enable_logging = true;
        self
    }

    /// Set the provider format
    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = provider;
        self
    }

    /// Use OpenAI API format
    pub fn openai(self) -> Self {
        self.provider(Provider::OpenAI)
    }

    /// Use Anthropic API format
    pub fn anthropic(self) -> Self {
        self.provider(Provider::Anthropic)
    }

    /// Use Google API format
    pub fn google(self) -> Self {
        self.provider(Provider::Google)
    }
}

/// Provider API format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    #[default]
    OpenAI,
    Anthropic,
    Google,
}

impl Provider {
    /// Get the provider name
    pub fn name(&self) -> &'static str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Google => "google",
        }
    }

    /// Get the chat endpoint path
    pub fn chat_endpoint(&self, model: &str) -> String {
        match self {
            Provider::OpenAI => "/v1/chat/completions".to_string(),
            Provider::Anthropic => "/v1/messages".to_string(),
            Provider::Google => format!("/v1/models/{}:generateContent", model),
        }
    }

    /// Get the embeddings endpoint path
    pub fn embeddings_endpoint(&self) -> &'static str {
        "/v1/embeddings"
    }

    /// Get the models endpoint path
    pub fn models_endpoint(&self) -> &'static str {
        "/v1/models"
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Provider::OpenAI),
            "anthropic" | "claude" => Ok(Provider::Anthropic),
            "google" | "gemini" => Ok(Provider::Google),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_builder() {
        let config = ClientConfig::new("http://example.com")
            .api_key("test-key")
            .timeout(Duration::from_secs(30))
            .max_retries(5)
            .default_model("gpt-4")
            .anthropic();

        assert_eq!(config.base_url, "http://example.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.default_model, Some("gpt-4".to_string()));
        assert_eq!(config.provider, Provider::Anthropic);
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!("anthropic".parse::<Provider>().unwrap(), Provider::Anthropic);
        assert_eq!("claude".parse::<Provider>().unwrap(), Provider::Anthropic);
        assert_eq!("google".parse::<Provider>().unwrap(), Provider::Google);
        assert_eq!("gemini".parse::<Provider>().unwrap(), Provider::Google);
    }
}
