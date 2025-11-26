//! Core simulation engine
//!
//! The SimulationEngine is the heart of the simulator, responsible for:
//! - Generating simulated responses
//! - Applying latency profiles
//! - Injecting errors for chaos testing
//! - Managing deterministic behavior with seeds

mod generator;
mod chaos;
mod state;

pub use generator::*;
pub use chaos::*;
pub use state::*;

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::config::{SimulatorConfig, ModelConfig};
use crate::error::{SimulationError, SimulatorResult};
use crate::latency::{LatencySimulator, LatencySchedule};
use crate::types::*;

/// The main simulation engine
pub struct SimulationEngine {
    config: Arc<RwLock<SimulatorConfig>>,
    latency_sim: LatencySimulator,
    chaos_engine: ChaosEngine,
    generator: ResponseGenerator,
    state: EngineState,
    start_time: Instant,
}

impl SimulationEngine {
    /// Create a new simulation engine with the given configuration
    pub fn new(config: SimulatorConfig) -> Self {
        let latency_sim = match config.seed {
            Some(seed) => LatencySimulator::with_seed(config.latency.clone(), seed),
            None => LatencySimulator::new(config.latency.clone()),
        };

        let chaos_engine = ChaosEngine::new(config.chaos.clone());
        let generator = ResponseGenerator::new(config.seed);

        Self {
            config: Arc::new(RwLock::new(config)),
            latency_sim,
            chaos_engine,
            generator,
            state: EngineState::new(),
            start_time: Instant::now(),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(SimulatorConfig::default())
    }

    /// Get current configuration
    pub fn config(&self) -> SimulatorConfig {
        self.config.read().clone()
    }

    /// Update configuration at runtime
    pub fn update_config(&mut self, config: SimulatorConfig) -> SimulatorResult<()> {
        config.validate()?;

        self.latency_sim = match config.seed {
            Some(seed) => LatencySimulator::with_seed(config.latency.clone(), seed),
            None => LatencySimulator::new(config.latency.clone()),
        };

        self.chaos_engine = ChaosEngine::new(config.chaos.clone());

        if let Some(seed) = config.seed {
            self.generator = ResponseGenerator::with_seed(seed);
        }

        *self.config.write() = config;
        Ok(())
    }

    /// Get engine uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate a chat completion response
    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> SimulatorResult<ChatCompletionResponse> {
        let start = Instant::now();
        self.state.increment_requests();

        // Check for chaos injection
        if let Some(error) = self.chaos_engine.maybe_inject_error(&request.model, "/chat/completions") {
            self.state.increment_errors();
            return Err(error);
        }

        // Validate model exists
        let model_config = self.get_model_config(&request.model)?;

        // Validate request
        request.validate().map_err(|e| SimulationError::Validation {
            message: e,
            param: None,
        })?;

        // Check context length
        let input_tokens = request.estimate_input_tokens();
        if input_tokens > model_config.context_length {
            return Err(SimulationError::ContextLengthExceeded {
                current: input_tokens,
                max: model_config.context_length,
            });
        }

        // Generate response
        let id = format!("chatcmpl-{}", Uuid::new_v4().to_string().replace("-", "")[..24].to_string());
        let max_tokens = request.effective_max_tokens().min(model_config.max_output_tokens as u32);

        let (content, output_tokens) = self.generator.generate_response(
            &request.messages,
            max_tokens,
            &model_config.generation,
        );

        let usage = Usage::new(input_tokens as u32, output_tokens);

        // Apply latency
        let profile = model_config.latency_profile.as_deref();
        let ttft = self.latency_sim.sample_ttft(profile);
        let overhead = self.latency_sim.overhead(profile);

        tokio::time::sleep(ttft + overhead).await;

        let response = ChatCompletionResponse::simple(id, request.model.clone(), content, usage);

        self.state.record_latency(start.elapsed());
        self.state.add_tokens(input_tokens as u64, output_tokens as u64);

        Ok(response)
    }

    /// Generate a streaming chat completion
    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> SimulatorResult<StreamingResponse> {
        self.state.increment_requests();

        // Check for chaos injection
        if let Some(error) = self.chaos_engine.maybe_inject_error(&request.model, "/chat/completions") {
            self.state.increment_errors();
            return Err(error);
        }

        // Validate model exists and supports streaming
        let model_config = self.get_model_config(&request.model)?;

        if !model_config.supports_streaming {
            return Err(SimulationError::Validation {
                message: format!("Model {} does not support streaming", request.model),
                param: Some("stream".to_string()),
            });
        }

        // Validate request
        request.validate().map_err(|e| SimulationError::Validation {
            message: e,
            param: None,
        })?;

        // Check context length
        let input_tokens = request.estimate_input_tokens();
        if input_tokens > model_config.context_length {
            return Err(SimulationError::ContextLengthExceeded {
                current: input_tokens,
                max: model_config.context_length,
            });
        }

        // Generate response tokens
        let id = format!("chatcmpl-{}", Uuid::new_v4().to_string().replace("-", "")[..24].to_string());
        let max_tokens = request.effective_max_tokens().min(model_config.max_output_tokens as u32);

        let (content, output_tokens) = self.generator.generate_response(
            &request.messages,
            max_tokens,
            &model_config.generation,
        );

        // Tokenize for streaming
        let tokens = self.generator.tokenize(&content);

        // Generate latency schedule
        let profile = model_config.latency_profile.as_deref();
        let schedule = self.latency_sim.generate_schedule(tokens.len(), profile);

        let usage = Usage::new(input_tokens as u32, output_tokens);
        self.state.add_tokens(input_tokens as u64, output_tokens as u64);

        Ok(StreamingResponse {
            id,
            model: request.model.clone(),
            tokens,
            schedule,
            usage,
        })
    }

    /// Generate embeddings
    pub async fn embeddings(&self, request: &EmbeddingsRequest) -> SimulatorResult<EmbeddingsResponse> {
        let start = Instant::now();
        self.state.increment_requests();

        // Check for chaos injection
        if let Some(error) = self.chaos_engine.maybe_inject_error(&request.model, "/embeddings") {
            self.state.increment_errors();
            return Err(error);
        }

        // Validate model exists and is an embedding model
        let model_config = self.get_model_config(&request.model)?;

        if !model_config.is_embedding {
            return Err(SimulationError::Validation {
                message: format!("Model {} is not an embedding model", request.model),
                param: Some("model".to_string()),
            });
        }

        let inputs = request.input.to_vec();
        let dimensions = request.dimensions
            .map(|d| d as usize)
            .or(model_config.embedding_dimensions)
            .unwrap_or(1536);

        let mut embeddings = Vec::with_capacity(inputs.len());
        let mut total_tokens = 0u32;

        for input in &inputs {
            let embedding = self.generator.generate_embedding(dimensions, input);
            let tokens = (input.len() / 4).max(1) as u32;
            total_tokens += tokens;
            embeddings.push(embedding);
        }

        // Apply latency
        let ttft = self.latency_sim.sample_ttft(None);
        tokio::time::sleep(ttft).await;

        let response = EmbeddingsResponse::new(request.model.clone(), embeddings, total_tokens);

        self.state.record_latency(start.elapsed());
        self.state.add_tokens(total_tokens as u64, 0);

        Ok(response)
    }

    /// List available models
    pub fn list_models(&self) -> ModelsResponse {
        let config = self.config.read();
        let models: Vec<ModelObject> = config.models.iter()
            .map(|(id, mc)| ModelObject::new(id, mc.provider.to_string()))
            .collect();
        ModelsResponse::new(models)
    }

    /// Get a specific model
    pub fn get_model(&self, model_id: &str) -> Option<ModelObject> {
        let config = self.config.read();
        config.models.get(model_id)
            .map(|mc| ModelObject::new(model_id, mc.provider.to_string()))
    }

    /// Get model configuration
    fn get_model_config(&self, model_id: &str) -> SimulatorResult<ModelConfig> {
        let config = self.config.read();
        config.models.get(model_id)
            .cloned()
            .ok_or_else(|| SimulationError::ModelNotFound(model_id.to_string()))
    }

    /// Get engine statistics
    pub fn stats(&self) -> EngineStats {
        self.state.stats()
    }

    /// Reset engine statistics
    pub fn reset_stats(&self) {
        self.state.reset();
    }

    /// Check if a model exists
    pub fn model_exists(&self, model_id: &str) -> bool {
        self.config.read().models.contains_key(model_id)
    }

    /// Get the latency simulator
    pub fn latency_simulator(&self) -> &LatencySimulator {
        &self.latency_sim
    }

    /// Get the chaos engine
    pub fn chaos_engine(&self) -> &ChaosEngine {
        &self.chaos_engine
    }
}

impl Clone for SimulationEngine {
    fn clone(&self) -> Self {
        let config = self.config.read().clone();
        Self::new(config)
    }
}

/// Response data for streaming
pub struct StreamingResponse {
    pub id: String,
    pub model: String,
    pub tokens: Vec<String>,
    pub schedule: LatencySchedule,
    pub usage: Usage,
}

impl StreamingResponse {
    /// Convert to SSE stream chunks
    pub fn into_chunks(self) -> Vec<(Duration, ChatCompletionChunk)> {
        let mut chunks = Vec::with_capacity(self.tokens.len() + 2);

        // First chunk with role
        let first_chunk = ChatCompletionChunk::new(
            self.id.clone(),
            self.model.clone(),
            vec![ChunkChoice::role_delta(Role::Assistant, 0)],
        );
        chunks.push((self.schedule.ttft + self.schedule.overhead, first_chunk));

        // Content chunks
        for (i, token) in self.tokens.iter().enumerate() {
            let chunk = ChatCompletionChunk::content_delta(
                self.id.clone(),
                self.model.clone(),
                token.clone(),
                0,
            );
            let delay = self.schedule.token_delays.get(i).copied().unwrap_or(Duration::ZERO);
            chunks.push((delay, chunk));
        }

        // Final chunk with finish reason
        let final_chunk = ChatCompletionChunk::finish(
            self.id.clone(),
            self.model.clone(),
            FinishReason::Stop,
            0,
        ).with_usage(self.usage);
        chunks.push((Duration::ZERO, final_chunk));

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = SimulationEngine::default_config();
        assert!(engine.model_exists("gpt-4"));
        assert!(engine.model_exists("claude-3-5-sonnet-20241022"));
    }

    #[tokio::test]
    async fn test_chat_completion() {
        let engine = SimulationEngine::default_config();
        let request = ChatCompletionRequest::new(
            "gpt-4",
            vec![Message::user("Hello!")],
        );

        let response = engine.chat_completion(&request).await.unwrap();
        assert!(!response.id.is_empty());
        assert_eq!(response.model, "gpt-4");
        assert!(!response.choices.is_empty());
    }

    #[tokio::test]
    async fn test_model_not_found() {
        let engine = SimulationEngine::default_config();
        let request = ChatCompletionRequest::new(
            "nonexistent-model",
            vec![Message::user("Hello!")],
        );

        let result = engine.chat_completion(&request).await;
        assert!(matches!(result, Err(SimulationError::ModelNotFound(_))));
    }

    #[tokio::test]
    async fn test_embeddings() {
        let engine = SimulationEngine::default_config();
        let request = EmbeddingsRequest {
            model: "text-embedding-ada-002".to_string(),
            input: EmbeddingInput::Single("Hello world".to_string()),
            encoding_format: None,
            dimensions: None,
            user: None,
        };

        let response = engine.embeddings(&request).await.unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding.len(), 1536);
    }

    #[tokio::test]
    async fn test_list_models() {
        let engine = SimulationEngine::default_config();
        let models = engine.list_models();

        assert!(!models.data.is_empty());
        assert!(models.data.iter().any(|m| m.id == "gpt-4"));
    }

    #[tokio::test]
    async fn test_streaming_response() {
        let engine = SimulationEngine::default_config();
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("Hello!")],
            stream: true,
            ..ChatCompletionRequest::new("gpt-4", vec![])
        };

        let response = engine.chat_completion_stream(&request).await.unwrap();
        assert!(!response.tokens.is_empty());

        let chunks = response.into_chunks();
        assert!(chunks.len() >= 2); // At least role + finish
    }

    #[tokio::test]
    async fn test_stats() {
        let engine = SimulationEngine::default_config();
        let request = ChatCompletionRequest::new(
            "gpt-4",
            vec![Message::user("Hello!")],
        );

        engine.chat_completion(&request).await.unwrap();

        let stats = engine.stats();
        assert_eq!(stats.total_requests, 1);
        assert!(stats.total_input_tokens > 0);
    }
}
