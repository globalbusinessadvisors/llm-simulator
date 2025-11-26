//! SDK Client
//!
//! The main client for interacting with LLM-Simulator instances.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use tracing::debug;

use crate::types::*;
use super::{ClientConfig, Provider, SdkError, SdkResult, ChatBuilder, EmbeddingsBuilder, StreamingChat};

/// LLM-Simulator SDK Client
///
/// The main entry point for interacting with an LLM-Simulator instance.
///
/// # Example
///
/// ```rust,no_run
/// use llm_simulator::sdk::Client;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = Client::new("http://localhost:8080")?;
///
///     // Simple chat completion
///     let response = client
///         .chat()
///         .model("gpt-4")
///         .message("Hello!")
///         .send()
///         .await?;
///
///     println!("{}", response.content());
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    config: Arc<ClientConfig>,
}

impl Client {
    /// Create a new client with the given base URL
    pub fn new(base_url: impl Into<String>) -> SdkResult<Self> {
        Self::with_config(ClientConfig::new(base_url))
    }

    /// Create a new client with the given configuration
    pub fn with_config(config: ClientConfig) -> SdkResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_str(&config.user_agent).unwrap_or_else(|_| HeaderValue::from_static("llm-simulator-sdk")));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(32)
            .build()
            .map_err(SdkError::Request)?;

        Ok(Self {
            http,
            config: Arc::new(config),
        })
    }

    /// Create a new client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Get the client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create a chat completion builder
    pub fn chat(&self) -> ChatBuilder {
        ChatBuilder::new(self.clone())
    }

    /// Create an embeddings builder
    pub fn embeddings(&self) -> EmbeddingsBuilder {
        EmbeddingsBuilder::new(self.clone())
    }

    /// Create a streaming chat builder
    pub fn stream(&self) -> StreamingChat {
        StreamingChat::new(self.clone())
    }

    /// List available models
    pub async fn list_models(&self) -> SdkResult<ModelsResponse> {
        let url = format!("{}{}", self.config.base_url, self.config.provider.models_endpoint());
        let response = self.execute_request(reqwest::Method::GET, &url, None::<&()>).await?;
        Ok(response)
    }

    /// Get a specific model
    pub async fn get_model(&self, model_id: &str) -> SdkResult<ModelObject> {
        let url = format!("{}{}/{}", self.config.base_url, self.config.provider.models_endpoint(), model_id);
        let response = self.execute_request(reqwest::Method::GET, &url, None::<&()>).await?;
        Ok(response)
    }

    /// Check health of the simulator instance
    pub async fn health(&self) -> SdkResult<HealthResponse> {
        let url = format!("{}/health", self.config.base_url);
        let response = self.execute_request(reqwest::Method::GET, &url, None::<&()>).await?;
        Ok(response)
    }

    /// Check readiness of the simulator instance
    pub async fn ready(&self) -> SdkResult<bool> {
        let url = format!("{}/ready", self.config.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Send a raw chat completion request
    pub async fn chat_completion(&self, request: &ChatCompletionRequest) -> SdkResult<ChatCompletionResponse> {
        let endpoint = self.config.provider.chat_endpoint(&request.model);
        let url = format!("{}{}", self.config.base_url, endpoint);

        match self.config.provider {
            Provider::OpenAI => {
                self.execute_request(reqwest::Method::POST, &url, Some(request)).await
            }
            Provider::Anthropic => {
                let anthropic_request = convert_to_anthropic(request);
                let response: serde_json::Value = self.execute_request(reqwest::Method::POST, &url, Some(&anthropic_request)).await?;
                convert_from_anthropic(response)
            }
            Provider::Google => {
                let google_request = convert_to_google(request);
                let response: serde_json::Value = self.execute_request(reqwest::Method::POST, &url, Some(&google_request)).await?;
                convert_from_google(response, &request.model)
            }
        }
    }

    /// Send a raw embeddings request
    pub async fn create_embeddings(&self, request: &EmbeddingsRequest) -> SdkResult<EmbeddingsResponse> {
        let url = format!("{}{}", self.config.base_url, self.config.provider.embeddings_endpoint());
        self.execute_request(reqwest::Method::POST, &url, Some(request)).await
    }

    /// Execute an HTTP request with retry logic
    pub(crate) async fn execute_request<T, R>(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&T>,
    ) -> SdkResult<R>
    where
        T: serde::Serialize + ?Sized,
        R: serde::de::DeserializeOwned,
    {
        let mut last_error: Option<SdkError> = None;
        let mut attempt = 0;

        while attempt <= self.config.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt, last_error.as_ref());
                debug!(attempt, delay_ms = delay.as_millis(), "Retrying request");
                tokio::time::sleep(delay).await;
            }

            let mut req = self.http.request(method.clone(), url);

            // Add authorization header if configured
            if let Some(ref key) = self.config.api_key {
                req = req.header(AUTHORIZATION, format!("Bearer {}", key));
            }

            // Add provider-specific headers
            if self.config.provider == Provider::Anthropic {
                req = req.header("anthropic-version", "2023-06-01");
                if let Some(ref key) = self.config.api_key {
                    req = req.header("x-api-key", key);
                }
            }

            // Add body if present
            if let Some(b) = body {
                req = req.json(b);
            }

            match req.send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        match response.json::<R>().await {
                            Ok(data) => return Ok(data),
                            Err(e) => {
                                last_error = Some(SdkError::Json(serde_json::Error::io(
                                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                                )));
                            }
                        }
                    } else {
                        let body: serde_json::Value = response.json().await.unwrap_or_default();
                        let err = SdkError::from_response(status.as_u16(), body);

                        if !err.is_retryable() {
                            return Err(err);
                        }

                        last_error = Some(err);
                    }
                }
                Err(e) => {
                    let err = SdkError::from(e);
                    if !err.is_retryable() {
                        return Err(err);
                    }
                    last_error = Some(err);
                }
            }

            attempt += 1;
        }

        Err(SdkError::RetryExhausted {
            attempts: self.config.max_retries + 1,
            last_error: Box::new(last_error.unwrap_or(SdkError::Timeout)),
        })
    }

    /// Calculate the retry delay with exponential backoff
    fn calculate_retry_delay(&self, attempt: u32, error: Option<&SdkError>) -> Duration {
        // Check for retry-after header
        if let Some(duration) = error.and_then(|e| e.retry_after()) {
            return duration.min(self.config.max_retry_delay);
        }

        // Exponential backoff with jitter
        let base = self.config.retry_delay.as_millis() as u64;
        let exp = base * 2u64.pow(attempt.saturating_sub(1));
        let jitter = rand::random::<u64>() % (exp / 2 + 1);
        let delay = Duration::from_millis(exp + jitter);

        delay.min(self.config.max_retry_delay)
    }

    /// Get internal HTTP client (for advanced use)
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }
}

/// Client builder for fluent configuration
#[derive(Default)]
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Set the timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the maximum retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.config.default_model = Some(model.into());
        self
    }

    /// Set the provider
    pub fn provider(mut self, provider: Provider) -> Self {
        self.config.provider = provider;
        self
    }

    /// Use OpenAI format
    pub fn openai(self) -> Self {
        self.provider(Provider::OpenAI)
    }

    /// Use Anthropic format
    pub fn anthropic(self) -> Self {
        self.provider(Provider::Anthropic)
    }

    /// Use Google format
    pub fn google(self) -> Self {
        self.provider(Provider::Google)
    }

    /// Enable logging
    pub fn enable_logging(mut self) -> Self {
        self.config.enable_logging = true;
        self
    }

    /// Build the client
    pub fn build(self) -> SdkResult<Client> {
        Client::with_config(self.config)
    }
}

/// Health response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub uptime_seconds: Option<u64>,
}

// Helper functions for provider conversion

fn convert_to_anthropic(request: &ChatCompletionRequest) -> serde_json::Value {
    let mut messages: Vec<serde_json::Value> = Vec::new();
    let mut system = None;

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                system = Some(msg.text());
            }
            Role::User => {
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": msg.text()
                }));
            }
            Role::Assistant => {
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": msg.text()
                }));
            }
            _ => {}
        }
    }

    let mut body = serde_json::json!({
        "model": request.model,
        "max_tokens": request.max_tokens.unwrap_or(256),
        "messages": messages
    });

    if let Some(sys) = system {
        body["system"] = serde_json::json!(sys);
    }

    if let Some(temp) = request.temperature {
        body["temperature"] = serde_json::json!(temp);
    }

    body
}

fn convert_from_anthropic(response: serde_json::Value) -> SdkResult<ChatCompletionResponse> {
    let content = response["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("")
        .to_string();

    let id = response["id"].as_str().unwrap_or("").to_string();
    let model = response["model"].as_str().unwrap_or("").to_string();

    let input_tokens = response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
    let output_tokens = response["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

    Ok(ChatCompletionResponse::simple(
        id,
        model,
        content,
        Usage::new(input_tokens, output_tokens),
    ))
}

fn convert_to_google(request: &ChatCompletionRequest) -> serde_json::Value {
    let contents: Vec<serde_json::Value> = request.messages.iter()
        .filter(|m| m.role != Role::System)
        .map(|m| {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "model",
                _ => "user",
            };
            serde_json::json!({
                "role": role,
                "parts": [{"text": m.text()}]
            })
        })
        .collect();

    let mut body = serde_json::json!({
        "contents": contents
    });

    if request.max_tokens.is_some() || request.temperature.is_some() {
        let mut config = serde_json::Map::new();
        if let Some(mt) = request.max_tokens {
            config.insert("maxOutputTokens".to_string(), serde_json::json!(mt));
        }
        if let Some(temp) = request.temperature {
            config.insert("temperature".to_string(), serde_json::json!(temp));
        }
        body["generationConfig"] = serde_json::Value::Object(config);
    }

    body
}

fn convert_from_google(response: serde_json::Value, model: &str) -> SdkResult<ChatCompletionResponse> {
    let content = response["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|parts| parts.first())
        .and_then(|p| p["text"].as_str())
        .unwrap_or("")
        .to_string();

    let input_tokens = response["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32;
    let output_tokens = response["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;

    Ok(ChatCompletionResponse::simple(
        format!("gen-{}", uuid::Uuid::new_v4()),
        model.to_string(),
        content,
        Usage::new(input_tokens, output_tokens),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new("http://localhost:8080").unwrap();
        assert_eq!(client.config().base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .base_url("http://example.com")
            .api_key("test-key")
            .timeout(Duration::from_secs(30))
            .max_retries(5)
            .openai()
            .build()
            .unwrap();

        assert_eq!(client.config().base_url, "http://example.com");
        assert_eq!(client.config().api_key, Some("test-key".to_string()));
        assert_eq!(client.config().timeout, Duration::from_secs(30));
        assert_eq!(client.config().max_retries, 5);
        assert_eq!(client.config().provider, Provider::OpenAI);
    }

    #[test]
    fn test_convert_to_anthropic() {
        let request = ChatCompletionRequest::new(
            "claude-3",
            vec![
                Message::system("You are helpful"),
                Message::user("Hello"),
            ],
        );

        let converted = convert_to_anthropic(&request);
        assert_eq!(converted["system"], "You are helpful");
        assert_eq!(converted["messages"][0]["role"], "user");
        assert_eq!(converted["messages"][0]["content"], "Hello");
    }
}
