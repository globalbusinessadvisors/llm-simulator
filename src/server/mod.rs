//! HTTP server implementation
//!
//! Provides the Axum-based HTTP server with all API endpoints
//! for OpenAI, Anthropic, and Google API compatibility.

mod routes;
mod middleware;
mod handlers;
mod state;
mod streaming;
pub mod shutdown;

pub use routes::*;
pub use handlers::*;
pub use state::*;
pub use streaming::*;
pub use shutdown::*;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::info;

use crate::config::SimulatorConfig;
use crate::engine::SimulationEngine;
use crate::security::{SecurityState, apply_security_middleware, build_cors_layer};
use crate::telemetry::{init_telemetry, SimulatorMetrics};

/// Run the simulator server
pub async fn run_server(config: SimulatorConfig) -> anyhow::Result<()> {
    // Initialize telemetry
    init_telemetry(&config.telemetry)?;

    // Create the simulation engine
    let engine = Arc::new(SimulationEngine::new(config.clone()));
    let metrics = Arc::new(SimulatorMetrics::new());

    // Create security state
    let security_state = SecurityState::new(&config.security);

    // Create shutdown state for graceful shutdown
    let shutdown_state = Arc::new(ShutdownState::new(config.server.request_timeout));

    // Create app state
    let state = AppState {
        engine,
        metrics,
        config: Arc::new(config.clone()),
        shutdown: shutdown_state.clone(),
    };

    // Build the router
    let app = create_router(state.clone(), security_state);

    // Build the server address
    let addr: SocketAddr = config.server.socket_addr();

    info!(
        "Starting LLM Simulator v{} on {}",
        env!("CARGO_PKG_VERSION"),
        addr
    );
    info!("Configured models: {}", config.models.len());
    info!("Latency simulation: {}", if config.latency.enabled { "enabled" } else { "disabled" });
    info!("Chaos engineering: {}", if config.chaos.enabled { "enabled" } else { "disabled" });
    info!("API key auth: {}", if config.security.api_keys.enabled { "enabled" } else { "disabled" });
    info!("Rate limiting: {}", if config.security.rate_limiting.enabled { "enabled" } else { "disabled" });

    // Create and run the server
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(graceful_shutdown(shutdown_state))
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Create the main router with all routes
pub fn create_router(state: AppState, security: SecurityState) -> Router {
    let config = state.config.clone();

    // Build CORS layer from security configuration
    let cors = build_cors_layer(&security.cors_config);

    // Build middleware stack (without security - that's applied separately)
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(config.server.request_timeout));

    // Build base router with all routes
    let base_router = Router::new()
        // OpenAI compatible routes
        .merge(routes::openai_routes())
        // Anthropic compatible routes
        .merge(routes::anthropic_routes())
        // Google compatible routes
        .merge(routes::google_routes())
        // Admin/utility routes
        .merge(routes::admin_routes())
        // Health and metrics
        .merge(routes::health_routes());

    // Apply security middleware (auth, rate limiting, headers)
    let secured_router = apply_security_middleware(base_router, security);

    // Apply remaining middleware and set state
    secured_router
        .layer(cors)
        .layer(middleware)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use crate::security::SecurityState;

    fn test_state() -> AppState {
        AppState::new(SimulatorConfig::default())
    }

    fn test_security() -> SecurityState {
        SecurityState::new(&crate::config::SecurityConfig::default())
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_router(test_state(), test_security());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_models_endpoint() {
        let app = create_router(test_state(), test_security());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
