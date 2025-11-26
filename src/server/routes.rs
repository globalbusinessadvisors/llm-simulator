//! Route definitions

use axum::{
    routing::{get, post},
    Router,
};

use super::handlers;
use super::state::AppState;

/// OpenAI compatible API routes
pub fn openai_routes() -> Router<AppState> {
    Router::new()
        // Chat completions
        .route("/v1/chat/completions", post(handlers::openai_chat_completions))
        // Embeddings
        .route("/v1/embeddings", post(handlers::openai_embeddings))
        // Models
        .route("/v1/models", get(handlers::list_models))
        .route("/v1/models/:model_id", get(handlers::get_model))
        // Legacy completions (maps to chat)
        .route("/v1/completions", post(handlers::openai_chat_completions))
}

/// Anthropic compatible API routes
pub fn anthropic_routes() -> Router<AppState> {
    Router::new()
        // Messages API
        .route("/v1/messages", post(handlers::anthropic_messages))
        // Also support without version prefix
        .route("/messages", post(handlers::anthropic_messages))
}

/// Google/Gemini compatible API routes
/// Note: Gemini uses paths like /v1/models/gemini-pro:generateContent
/// We use a slightly different pattern since axum doesn't support `:action` suffix
pub fn google_routes() -> Router<AppState> {
    Router::new()
        // Generate content - using separate path segments
        .route(
            "/v1beta/models/:model_id/generateContent",
            post(handlers::gemini_generate_content),
        )
        .route(
            "/v1/models/:model_id/generateContent",
            post(handlers::gemini_generate_content),
        )
        // Stream generate content
        .route(
            "/v1beta/models/:model_id/streamGenerateContent",
            post(handlers::gemini_stream_generate_content),
        )
        .route(
            "/v1/models/:model_id/streamGenerateContent",
            post(handlers::gemini_stream_generate_content),
        )
}

/// Admin and configuration routes
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        // Statistics
        .route("/admin/stats", get(handlers::get_stats))
        .route("/admin/stats/reset", post(handlers::reset_stats))
        // Configuration
        .route("/admin/config", get(handlers::get_config))
        .route("/admin/config", post(handlers::update_config))
        // Chaos engineering
        .route("/admin/chaos/enable", post(handlers::enable_chaos))
        .route("/admin/chaos/disable", post(handlers::disable_chaos))
        .route("/admin/chaos/status", get(handlers::chaos_status))
}

/// Health and metrics routes
pub fn health_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        .route("/healthz", get(handlers::health_check))
        .route("/ready", get(handlers::ready_check))
        .route("/readyz", get(handlers::ready_check))
        // Metrics (Prometheus format)
        .route("/metrics", get(handlers::metrics))
        // Version info
        .route("/version", get(handlers::version))
        // Root
        .route("/", get(handlers::root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimulatorConfig;

    fn test_state() -> AppState {
        AppState::new(SimulatorConfig::default())
    }

    #[test]
    fn test_routes_compile() {
        let _openai = openai_routes();
        let _anthropic = anthropic_routes();
        let _google = google_routes();
        let _admin = admin_routes();
        let _health = health_routes();
    }

    #[test]
    fn test_router_creation() {
        let state = test_state();
        let router: Router<AppState> = Router::new()
            .merge(openai_routes())
            .merge(anthropic_routes())
            .merge(google_routes())
            .merge(admin_routes())
            .merge(health_routes())
            .with_state(state);

        // Router should compile without issues
        let _ = router;
    }
}
