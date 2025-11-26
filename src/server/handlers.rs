//! HTTP request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use serde::Serialize;
use std::time::Duration;

use super::state::AppState;
use super::streaming::create_sse_stream;
use crate::config::SimulatorConfig;
use crate::engine::EngineStats;
use crate::error::SimulationError;
use crate::types::*;

// ============== OpenAI Handlers ==============

/// POST /v1/chat/completions
pub async fn openai_chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, SimulationError> {
    if request.stream {
        // Streaming response
        let stream_response = state.engine.chat_completion_stream(&request).await?;
        let stream = create_sse_stream(stream_response);
        Ok(Sse::new(stream)
            .keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text("keep-alive"),
            )
            .into_response())
    } else {
        // Non-streaming response
        let response = state.engine.chat_completion(&request).await?;
        Ok(Json(response).into_response())
    }
}

/// POST /v1/embeddings
pub async fn openai_embeddings(
    State(state): State<AppState>,
    Json(request): Json<EmbeddingsRequest>,
) -> Result<Json<EmbeddingsResponse>, SimulationError> {
    let response = state.engine.embeddings(&request).await?;
    Ok(Json(response))
}

/// GET /v1/models
pub async fn list_models(
    State(state): State<AppState>,
) -> Json<ModelsResponse> {
    Json(state.engine.list_models())
}

/// GET /v1/models/:model_id
pub async fn get_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<Json<ModelObject>, SimulationError> {
    state.engine.get_model(&model_id)
        .map(Json)
        .ok_or_else(|| SimulationError::ModelNotFound(model_id))
}

// ============== Anthropic Handlers ==============

/// POST /v1/messages
pub async fn anthropic_messages(
    State(state): State<AppState>,
    Json(request): Json<AnthropicMessagesRequest>,
) -> Result<Response, SimulationError> {
    // Convert to internal format
    let messages = request.messages.iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => Role::User,
            };
            Message {
                role,
                content: MessageContent::Text(m.content.text()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                function_call: None,
            }
        })
        .collect::<Vec<_>>();

    // Add system message if present
    let mut all_messages = Vec::new();
    if let Some(system) = &request.system {
        all_messages.push(Message::system(system.clone()));
    }
    all_messages.extend(messages);

    let chat_request = ChatCompletionRequest {
        model: request.model.clone(),
        messages: all_messages,
        temperature: request.temperature,
        top_p: request.top_p,
        max_tokens: Some(request.max_tokens),
        stream: request.stream,
        ..ChatCompletionRequest::new(&request.model, vec![])
    };

    if request.stream {
        let stream_response = state.engine.chat_completion_stream(&chat_request).await?;
        let stream = super::streaming::create_anthropic_sse_stream(stream_response, &request.model);
        Ok(Sse::new(stream)
            .keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text("ping"),
            )
            .into_response())
    } else {
        let response = state.engine.chat_completion(&chat_request).await?;

        // Convert to Anthropic format
        let content = response.choices.first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let anthropic_response = AnthropicMessagesResponse::new(
            response.id,
            request.model,
            content,
            response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            response.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
        );

        Ok(Json(anthropic_response).into_response())
    }
}

// ============== Google/Gemini Handlers ==============

/// POST /v1/models/:model_id:generateContent
pub async fn gemini_generate_content(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
    Json(request): Json<GeminiRequest>,
) -> Result<Json<GeminiResponse>, SimulationError> {
    // Convert to internal format
    let messages = request.contents.iter()
        .map(|c| {
            let role = match c.role.as_str() {
                "user" => Role::User,
                "model" => Role::Assistant,
                _ => Role::User,
            };
            let content = c.parts.iter()
                .filter_map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("");
            Message {
                role,
                content: MessageContent::Text(content),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                function_call: None,
            }
        })
        .collect::<Vec<_>>();

    let max_tokens = request.generation_config
        .as_ref()
        .and_then(|c| c.max_output_tokens)
        .unwrap_or(4096);

    let chat_request = ChatCompletionRequest {
        model: model_id.clone(),
        messages,
        temperature: request.generation_config.as_ref().and_then(|c| c.temperature),
        top_p: request.generation_config.as_ref().and_then(|c| c.top_p),
        max_tokens: Some(max_tokens),
        stream: false,
        ..ChatCompletionRequest::new(&model_id, vec![])
    };

    let response = state.engine.chat_completion(&chat_request).await?;

    let content = response.choices.first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    let gemini_response = GeminiResponse::new(
        content,
        response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
        response.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
    );

    Ok(Json(gemini_response))
}

/// POST /v1/models/:model_id:streamGenerateContent
pub async fn gemini_stream_generate_content(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
    Json(request): Json<GeminiRequest>,
) -> Result<Response, SimulationError> {
    // Convert to internal format (similar to above)
    let messages = request.contents.iter()
        .map(|c| {
            let role = match c.role.as_str() {
                "user" => Role::User,
                "model" => Role::Assistant,
                _ => Role::User,
            };
            let content = c.parts.iter()
                .filter_map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("");
            Message {
                role,
                content: MessageContent::Text(content),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                function_call: None,
            }
        })
        .collect();

    let max_tokens = request.generation_config
        .as_ref()
        .and_then(|c| c.max_output_tokens)
        .unwrap_or(4096);

    let chat_request = ChatCompletionRequest {
        model: model_id.clone(),
        messages,
        temperature: request.generation_config.as_ref().and_then(|c| c.temperature),
        max_tokens: Some(max_tokens),
        stream: true,
        ..ChatCompletionRequest::new(&model_id, vec![])
    };

    let stream_response = state.engine.chat_completion_stream(&chat_request).await?;
    let stream = super::streaming::create_gemini_sse_stream(stream_response);

    Ok(Sse::new(stream).into_response())
}

// ============== Admin Handlers ==============

/// GET /admin/stats
pub async fn get_stats(
    State(state): State<AppState>,
) -> Json<EngineStats> {
    Json(state.engine.stats())
}

/// POST /admin/stats/reset
pub async fn reset_stats(
    State(state): State<AppState>,
) -> StatusCode {
    state.engine.reset_stats();
    StatusCode::NO_CONTENT
}

/// GET /admin/config
pub async fn get_config(
    State(state): State<AppState>,
) -> Json<SimulatorConfig> {
    Json(state.engine.config())
}

/// POST /admin/config
pub async fn update_config(
    State(_state): State<AppState>,
    Json(_config): Json<SimulatorConfig>,
) -> Result<StatusCode, SimulationError> {
    // Note: Runtime config update requires mutable access
    // For now, return not implemented
    Err(SimulationError::Internal(
        "Runtime config updates not yet supported".to_string()
    ))
}

/// POST /admin/chaos/enable
pub async fn enable_chaos(
    State(_state): State<AppState>,
) -> StatusCode {
    // Would need mutable access to engine
    StatusCode::NOT_IMPLEMENTED
}

/// POST /admin/chaos/disable
pub async fn disable_chaos(
    State(_state): State<AppState>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /admin/chaos/status
pub async fn chaos_status(
    State(state): State<AppState>,
) -> Json<ChaosStatusResponse> {
    Json(ChaosStatusResponse {
        enabled: state.config.chaos.enabled,
        global_probability: state.config.chaos.global_probability,
        active_rules: state.config.chaos.errors.len(),
        circuit_breaker_enabled: state.config.chaos.circuit_breaker.enabled,
        rate_limiting_enabled: state.config.chaos.rate_limiting.enabled,
    })
}

#[derive(Serialize)]
pub struct ChaosStatusResponse {
    pub enabled: bool,
    pub global_probability: f64,
    pub active_rules: usize,
    pub circuit_breaker_enabled: bool,
    pub rate_limiting_enabled: bool,
}

// ============== Health Handlers ==============

/// GET /health
pub async fn health_check(
    State(state): State<AppState>,
) -> Json<DetailedHealthResponse> {
    let mut checks = std::collections::HashMap::new();
    let mut overall_status = HealthStatus::Healthy;

    // Check 1: Engine initialization
    let engine_check = check_engine(&state);
    if engine_check.status == ComponentStatus::Fail {
        overall_status = HealthStatus::Unhealthy;
    }
    checks.insert("engine".to_string(), engine_check);

    // Check 2: Configuration validity
    let config_check = check_config(&state);
    if config_check.status == ComponentStatus::Fail {
        overall_status = HealthStatus::Unhealthy;
    } else if config_check.status == ComponentStatus::Warn && overall_status == HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }
    checks.insert("config".to_string(), config_check);

    // Check 3: Metrics subsystem
    let metrics_check = check_metrics(&state);
    if metrics_check.status == ComponentStatus::Warn && overall_status == HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }
    checks.insert("metrics".to_string(), metrics_check);

    // Check 4: Memory usage
    let memory_check = check_memory();
    if memory_check.status == ComponentStatus::Warn && overall_status == HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }
    checks.insert("memory".to_string(), memory_check);

    // Check 5: Shutdown state
    if state.shutdown.is_draining() {
        overall_status = HealthStatus::Unhealthy;
        checks.insert("shutdown".to_string(), ComponentHealth {
            status: ComponentStatus::Fail,
            message: Some("Server is draining".to_string()),
            latency_ms: None,
            value: None,
        });
    }

    Json(DetailedHealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.engine.uptime().as_secs(),
        timestamp: chrono::Utc::now(),
        checks,
    })
}

/// GET /ready
pub async fn ready_check(
    State(state): State<AppState>,
) -> (StatusCode, Json<ReadyResponse>) {
    // Not ready if draining
    if state.shutdown.is_draining() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                ready: false,
                reason: Some("Server is draining".to_string()),
            }),
        );
    }

    // Check if engine is ready
    let engine_ready = true; // Engine is always ready once started
    if !engine_ready {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                ready: false,
                reason: Some("Engine not initialized".to_string()),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(ReadyResponse {
            ready: true,
            reason: None,
        }),
    )
}

/// GET /metrics
pub async fn metrics(
    State(state): State<AppState>,
) -> String {
    // Update queue metrics before export
    state.metrics.set_queue_depth(state.shutdown.in_flight_count());
    state.metrics.export()
}

// Health check helper functions
fn check_engine(state: &AppState) -> ComponentHealth {
    let start = std::time::Instant::now();
    let stats = state.engine.stats();
    let latency = start.elapsed().as_millis() as u64;

    ComponentHealth {
        status: ComponentStatus::Pass,
        message: Some(format!("Processed {} requests", stats.total_requests)),
        latency_ms: Some(latency),
        value: Some(stats.total_requests as f64),
    }
}

fn check_config(state: &AppState) -> ComponentHealth {
    let model_count = state.config.models.len();

    if model_count == 0 {
        ComponentHealth {
            status: ComponentStatus::Warn,
            message: Some("No models configured".to_string()),
            latency_ms: None,
            value: Some(0.0),
        }
    } else {
        ComponentHealth {
            status: ComponentStatus::Pass,
            message: Some(format!("{} models configured", model_count)),
            latency_ms: None,
            value: Some(model_count as f64),
        }
    }
}

fn check_metrics(_state: &AppState) -> ComponentHealth {
    // Metrics are always available (in-process)
    ComponentHealth {
        status: ComponentStatus::Pass,
        message: Some("Metrics available".to_string()),
        latency_ms: None,
        value: None,
    }
}

fn check_memory() -> ComponentHealth {
    // Simple memory check - in production you'd use actual memory stats
    // For now, always pass
    ComponentHealth {
        status: ComponentStatus::Pass,
        message: Some("Memory usage normal".to_string()),
        latency_ms: None,
        value: None,
    }
}

/// Detailed health response following health check RFC
#[derive(Debug, Clone, Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checks: std::collections::HashMap<String, ComponentHealth>,
}

/// Overall health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual component health
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: ComponentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// Component status (pass/warn/fail)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Pass,
    Warn,
    Fail,
}

/// Readiness response
#[derive(Debug, Clone, Serialize)]
pub struct ReadyResponse {
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// GET /version
pub async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
    })
}

#[derive(Serialize)]
pub struct VersionResponse {
    pub name: String,
    pub version: String,
    pub rust_version: String,
}

/// GET /
pub async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        name: "LLM Simulator".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: env!("CARGO_PKG_DESCRIPTION").to_string(),
        endpoints: vec![
            "/v1/chat/completions".to_string(),
            "/v1/embeddings".to_string(),
            "/v1/models".to_string(),
            "/v1/messages".to_string(),
            "/health".to_string(),
            "/metrics".to_string(),
        ],
    })
}

#[derive(Serialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub endpoints: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_response() {
        let response = VersionResponse {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            rust_version: "1.75".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_chaos_status_response() {
        let response = ChaosStatusResponse {
            enabled: true,
            global_probability: 0.5,
            active_rules: 3,
            circuit_breaker_enabled: true,
            rate_limiting_enabled: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"enabled\":true"));
    }
}
