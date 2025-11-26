//! Graceful Shutdown Implementation
//!
//! Provides connection draining and graceful shutdown functionality:
//! - Tracks in-flight requests
//! - Rejects new requests during drain
//! - Waits for existing requests to complete
//! - Supports configurable drain timeout

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{info, warn};

use crate::error::ErrorResponse;

/// Shutdown state for tracking in-flight requests
#[derive(Debug)]
pub struct ShutdownState {
    /// Number of in-flight requests
    in_flight: AtomicU64,
    /// Whether we're in draining mode
    draining: AtomicBool,
    /// Whether server is ready to accept requests
    ready: AtomicBool,
    /// Drain timeout
    drain_timeout: Duration,
    /// Server start time
    start_time: Instant,
}

impl ShutdownState {
    /// Create new shutdown state
    pub fn new(drain_timeout: Duration) -> Self {
        Self {
            in_flight: AtomicU64::new(0),
            draining: AtomicBool::new(false),
            ready: AtomicBool::new(true),
            drain_timeout,
            start_time: Instant::now(),
        }
    }

    /// Mark a request as started
    pub fn request_started(&self) {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark a request as completed
    pub fn request_completed(&self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get the number of in-flight requests
    pub fn in_flight_count(&self) -> u64 {
        self.in_flight.load(Ordering::SeqCst)
    }

    /// Check if we're draining
    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    /// Check if server is ready
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst) && !self.is_draining()
    }

    /// Start draining (stop accepting new requests)
    pub fn start_drain(&self) {
        info!("Starting graceful shutdown, marking as draining");
        self.draining.store(true, Ordering::SeqCst);
        self.ready.store(false, Ordering::SeqCst);
    }

    /// Set readiness state
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get drain timeout
    pub fn drain_timeout(&self) -> Duration {
        self.drain_timeout
    }

    /// Wait for all in-flight requests to complete
    pub async fn wait_for_drain(&self) {
        let drain_start = Instant::now();

        while self.in_flight_count() > 0 {
            if drain_start.elapsed() > self.drain_timeout {
                warn!(
                    remaining_requests = self.in_flight_count(),
                    "Drain timeout exceeded, forcing shutdown"
                );
                break;
            }

            info!(
                in_flight = self.in_flight_count(),
                elapsed_ms = drain_start.elapsed().as_millis() as u64,
                "Waiting for in-flight requests to complete"
            );

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("All requests drained, proceeding with shutdown");
    }
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

/// Drain status response
#[derive(Debug, Serialize)]
pub struct DrainStatus {
    pub draining: bool,
    pub in_flight_requests: u64,
    pub ready: bool,
    pub uptime_seconds: u64,
}

/// Request tracking middleware
pub async fn request_tracking_middleware(
    State(shutdown): State<Arc<ShutdownState>>,
    request: Request,
    next: Next,
) -> Result<Response, DrainError> {
    // Reject new requests if draining
    if shutdown.is_draining() {
        return Err(DrainError);
    }

    // Track request start
    shutdown.request_started();

    // Execute request
    let response = next.run(request).await;

    // Track request completion
    shutdown.request_completed();

    Ok(response)
}

/// Error returned when server is draining
#[derive(Debug)]
pub struct DrainError;

impl IntoResponse for DrainError {
    fn into_response(self) -> Response {
        let body = ErrorResponse::new(
            "service_unavailable",
            "Server is shutting down. Please retry your request.",
        );
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

/// Graceful shutdown handler
pub async fn graceful_shutdown(shutdown_state: Arc<ShutdownState>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown");
        }
        _ = terminate => {
            info!("Received SIGTERM, starting graceful shutdown");
        }
    }

    // Start draining
    shutdown_state.start_drain();

    // Wait for in-flight requests
    shutdown_state.wait_for_drain().await;
}

/// Admin endpoint to trigger drain manually
pub async fn admin_drain(
    State(shutdown): State<Arc<ShutdownState>>,
) -> Json<DrainStatus> {
    shutdown.start_drain();

    Json(DrainStatus {
        draining: shutdown.is_draining(),
        in_flight_requests: shutdown.in_flight_count(),
        ready: shutdown.is_ready(),
        uptime_seconds: shutdown.uptime().as_secs(),
    })
}

/// Admin endpoint to get drain status
pub async fn admin_drain_status(
    State(shutdown): State<Arc<ShutdownState>>,
) -> Json<DrainStatus> {
    Json(DrainStatus {
        draining: shutdown.is_draining(),
        in_flight_requests: shutdown.in_flight_count(),
        ready: shutdown.is_ready(),
        uptime_seconds: shutdown.uptime().as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_state_creation() {
        let state = ShutdownState::new(Duration::from_secs(30));
        assert_eq!(state.in_flight_count(), 0);
        assert!(!state.is_draining());
        assert!(state.is_ready());
    }

    #[test]
    fn test_request_tracking() {
        let state = ShutdownState::new(Duration::from_secs(30));

        state.request_started();
        assert_eq!(state.in_flight_count(), 1);

        state.request_started();
        assert_eq!(state.in_flight_count(), 2);

        state.request_completed();
        assert_eq!(state.in_flight_count(), 1);

        state.request_completed();
        assert_eq!(state.in_flight_count(), 0);
    }

    #[test]
    fn test_drain_mode() {
        let state = ShutdownState::new(Duration::from_secs(30));

        assert!(!state.is_draining());
        assert!(state.is_ready());

        state.start_drain();

        assert!(state.is_draining());
        assert!(!state.is_ready());
    }

    #[test]
    fn test_drain_status_serialization() {
        let status = DrainStatus {
            draining: true,
            in_flight_requests: 5,
            ready: false,
            uptime_seconds: 3600,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"draining\":true"));
        assert!(json.contains("\"in_flight_requests\":5"));
    }
}
