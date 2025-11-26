//! Request Builders
//!
//! Fluent builder patterns for constructing API requests.

use crate::types::*;
use super::{Client, SdkError, SdkResult};

/// Chat completion request builder
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
///     let response = client
///         .chat()
///         .model("gpt-4")
///         .system("You are a helpful assistant.")
///         .message("What is the capital of France?")
///         .temperature(0.7)
///         .max_tokens(100)
///         .send()
///         .await?;
///
///     println!("{}", response.content());
///     Ok(())
/// }
/// ```
pub struct ChatBuilder {
    client: Client,
    model: Option<String>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<Vec<String>>,
    user: Option<String>,
    stream: bool,
}

impl ChatBuilder {
    /// Create a new chat builder
    pub fn new(client: Client) -> Self {
        let default_model = client.config().default_model.clone();
        let default_temp = client.config().default_temperature;
        let default_tokens = client.config().default_max_tokens;

        Self {
            client,
            model: default_model,
            messages: Vec::new(),
            temperature: default_temp,
            max_tokens: default_tokens,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
            stream: false,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content.into()));
        self
    }

    /// Add a user message
    pub fn message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content.into()));
        self
    }

    /// Add a user message (alias for message)
    pub fn user_message(self, content: impl Into<String>) -> Self {
        self.message(content)
    }

    /// Add an assistant message
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content.into()));
        self
    }

    /// Add a raw message
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set the temperature (0.0-2.0)
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum tokens to generate
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set the top_p parameter
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set the frequency penalty
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences
    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
        self
    }

    /// Add a stop sequence
    pub fn add_stop(mut self, sequence: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(sequence.into());
        self
    }

    /// Set the user identifier
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Enable streaming
    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Build the request without sending
    pub fn build(self) -> SdkResult<ChatCompletionRequest> {
        let model = self.model.ok_or_else(|| {
            SdkError::InvalidRequest("Model is required".to_string())
        })?;

        if self.messages.is_empty() {
            return Err(SdkError::InvalidRequest("At least one message is required".to_string()));
        }

        let stop = self.stop.map(|s| {
            if s.len() == 1 {
                StopSequence::Single(s.into_iter().next().unwrap())
            } else {
                StopSequence::Multiple(s)
            }
        });

        Ok(ChatCompletionRequest::new(model, self.messages.clone())
            .with_options(
                self.temperature,
                self.top_p,
                self.max_tokens,
                self.stream,
                self.frequency_penalty,
                self.presence_penalty,
                stop,
                self.user,
            ))
    }

    /// Send the request and get the response
    pub async fn send(self) -> SdkResult<ChatResponse> {
        let client = self.client.clone();
        let request = self.build()?;
        let response = client.chat_completion(&request).await?;
        Ok(ChatResponse::new(response))
    }
}

/// Embeddings request builder
pub struct EmbeddingsBuilder {
    client: Client,
    model: Option<String>,
    input: Vec<String>,
    dimensions: Option<u32>,
    encoding_format: Option<String>,
    user: Option<String>,
}

impl EmbeddingsBuilder {
    /// Create a new embeddings builder
    pub fn new(client: Client) -> Self {
        Self {
            client,
            model: Some("text-embedding-ada-002".to_string()),
            input: Vec::new(),
            dimensions: None,
            encoding_format: None,
            user: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the input text
    pub fn input(mut self, text: impl Into<String>) -> Self {
        self.input.push(text.into());
        self
    }

    /// Set multiple input texts
    pub fn inputs(mut self, texts: Vec<String>) -> Self {
        self.input.extend(texts);
        self
    }

    /// Set the embedding dimensions
    pub fn dimensions(mut self, dims: u32) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Set the encoding format
    pub fn encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Set the user identifier
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Build the request without sending
    pub fn build(self) -> SdkResult<EmbeddingsRequest> {
        let model = self.model.ok_or_else(|| {
            SdkError::InvalidRequest("Model is required".to_string())
        })?;

        if self.input.is_empty() {
            return Err(SdkError::InvalidRequest("At least one input is required".to_string()));
        }

        let input = if self.input.len() == 1 {
            EmbeddingInput::Single(self.input.into_iter().next().unwrap())
        } else {
            EmbeddingInput::Multiple(self.input)
        };

        Ok(EmbeddingsRequest {
            model,
            input,
            encoding_format: self.encoding_format,
            dimensions: self.dimensions,
            user: self.user,
        })
    }

    /// Send the request and get the response
    pub async fn send(self) -> SdkResult<EmbeddingsResult> {
        let client = self.client.clone();
        let request = self.build()?;
        let response = client.create_embeddings(&request).await?;
        Ok(EmbeddingsResult::new(response))
    }
}

/// Wrapper around ChatCompletionResponse with convenience methods
pub struct ChatResponse {
    inner: ChatCompletionResponse,
}

impl ChatResponse {
    pub fn new(response: ChatCompletionResponse) -> Self {
        Self { inner: response }
    }

    /// Get the response content
    pub fn content(&self) -> &str {
        self.inner.choices.first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
    }

    /// Get the full content or empty string
    pub fn text(&self) -> String {
        self.content().to_string()
    }

    /// Get the response ID
    pub fn id(&self) -> &str {
        &self.inner.id
    }

    /// Get the model used
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// Get usage information
    pub fn usage(&self) -> Option<&Usage> {
        self.inner.usage.as_ref()
    }

    /// Get the finish reason
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.inner.choices.first().and_then(|c| c.finish_reason)
    }

    /// Get all choices
    pub fn choices(&self) -> &[ChatCompletionChoice] {
        &self.inner.choices
    }

    /// Get the number of choices
    pub fn num_choices(&self) -> usize {
        self.inner.choices.len()
    }

    /// Get input tokens used
    pub fn input_tokens(&self) -> u32 {
        self.inner.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0)
    }

    /// Get output tokens used
    pub fn output_tokens(&self) -> u32 {
        self.inner.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0)
    }

    /// Get total tokens used
    pub fn total_tokens(&self) -> u32 {
        self.inner.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0)
    }

    /// Check if the response was truncated
    pub fn is_truncated(&self) -> bool {
        matches!(self.finish_reason(), Some(FinishReason::Length))
    }

    /// Get the inner response
    pub fn into_inner(self) -> ChatCompletionResponse {
        self.inner
    }

    /// Get reference to inner response
    pub fn inner(&self) -> &ChatCompletionResponse {
        &self.inner
    }
}

/// Wrapper around EmbeddingsResponse with convenience methods
pub struct EmbeddingsResult {
    inner: EmbeddingsResponse,
}

impl EmbeddingsResult {
    pub fn new(response: EmbeddingsResponse) -> Self {
        Self { inner: response }
    }

    /// Get the first embedding
    pub fn embedding(&self) -> Option<&[f32]> {
        self.inner.data.first().map(|e| e.embedding.as_slice())
    }

    /// Get all embeddings
    pub fn embeddings(&self) -> Vec<&[f32]> {
        self.inner.data.iter().map(|e| e.embedding.as_slice()).collect()
    }

    /// Get the number of embeddings
    pub fn count(&self) -> usize {
        self.inner.data.len()
    }

    /// Get the embedding dimensions
    pub fn dimensions(&self) -> usize {
        self.inner.data.first()
            .map(|e| e.embedding.len())
            .unwrap_or(0)
    }

    /// Get usage information
    pub fn usage(&self) -> &EmbeddingUsage {
        &self.inner.usage
    }

    /// Get total tokens used
    pub fn total_tokens(&self) -> u32 {
        self.inner.usage.total_tokens
    }

    /// Get the model used
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Get the inner response
    pub fn into_inner(self) -> EmbeddingsResponse {
        self.inner
    }

    /// Get reference to inner response
    pub fn inner(&self) -> &EmbeddingsResponse {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_builder_validation() {
        let client = Client::new("http://localhost:8080").unwrap();

        // Missing model - builder has no default model in this test
        let builder = ChatBuilder::new(client.clone());
        assert!(builder.model.is_none());

        // Missing messages
        let result = client.chat()
            .model("gpt-4")
            .build();
        assert!(result.is_err());

        // Valid request
        let result = client.chat()
            .model("gpt-4")
            .message("Hello")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_embeddings_builder_validation() {
        let client = Client::new("http://localhost:8080").unwrap();

        // Missing input
        let result = client.embeddings()
            .model("text-embedding-ada-002")
            .build();
        assert!(result.is_err());

        // Valid request
        let result = client.embeddings()
            .model("text-embedding-ada-002")
            .input("Hello, world!")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((EmbeddingsResult::cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((EmbeddingsResult::cosine_similarity(&a, &c) - 0.0).abs() < 0.0001);
    }
}
