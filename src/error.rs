//! Error types for LLM-Simulator

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Result type alias for simulator operations
pub type SimulatorResult<T> = Result<T, SimulationError>;

/// Main error type for simulation operations
#[derive(Error, Debug, Clone)]
pub enum SimulationError {
    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {message}")]
    Validation {
        message: String,
        param: Option<String>,
    },

    // Provider errors
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    // Rate limiting
    #[error("Rate limit exceeded")]
    RateLimitExceeded { retry_after: Duration },

    // Timeout
    #[error("Request timeout after {0:?}")]
    Timeout(Duration),

    // Authentication
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    // Service errors
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),

    // Injected errors for chaos testing
    #[error("Simulated error: {error_type}")]
    Injected {
        error_type: InjectedErrorType,
        message: String,
        status_code: u16,
    },

    // Session errors
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Context length exceeded: {current} > {max}")]
    ContextLengthExceeded { current: usize, max: usize },

    // Streaming errors
    #[error("Stream error: {0}")]
    StreamError(String),
}

/// Types of injected errors for chaos engineering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectedErrorType {
    RateLimit,
    Timeout,
    ServerError,
    BadGateway,
    ServiceUnavailable,
    AuthenticationError,
    InvalidRequest,
    ContextLengthExceeded,
}

impl std::fmt::Display for InjectedErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimit => write!(f, "rate_limit_exceeded"),
            Self::Timeout => write!(f, "timeout"),
            Self::ServerError => write!(f, "server_error"),
            Self::BadGateway => write!(f, "bad_gateway"),
            Self::ServiceUnavailable => write!(f, "service_unavailable"),
            Self::AuthenticationError => write!(f, "authentication_error"),
            Self::InvalidRequest => write!(f, "invalid_request_error"),
            Self::ContextLengthExceeded => write!(f, "context_length_exceeded"),
        }
    }
}

/// OpenAI-compatible error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ErrorResponse {
    pub fn new(error_type: &str, message: &str) -> Self {
        Self {
            error: ErrorDetail {
                message: message.to_string(),
                error_type: error_type.to_string(),
                param: None,
                code: None,
            },
        }
    }

    pub fn with_param(mut self, param: &str) -> Self {
        self.error.param = Some(param.to_string());
        self
    }

    pub fn with_code(mut self, code: &str) -> Self {
        self.error.code = Some(code.to_string());
        self
    }
}

impl SimulationError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Validation { .. } => StatusCode::BAD_REQUEST,
            Self::ProviderNotFound(_) | Self::ModelNotFound(_) => StatusCode::NOT_FOUND,
            Self::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            Self::AuthenticationFailed(_) => StatusCode::UNAUTHORIZED,
            Self::PermissionDenied(_) => StatusCode::FORBIDDEN,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Injected { status_code, .. } => {
                StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::SessionNotFound(_) => StatusCode::NOT_FOUND,
            Self::ContextLengthExceeded { .. } => StatusCode::BAD_REQUEST,
            Self::StreamError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_type(&self) -> &str {
        match self {
            Self::Config(_) => "configuration_error",
            Self::Validation { .. } => "invalid_request_error",
            Self::ProviderNotFound(_) | Self::ModelNotFound(_) => "not_found_error",
            Self::RateLimitExceeded { .. } => "rate_limit_error",
            Self::Timeout(_) => "timeout_error",
            Self::AuthenticationFailed(_) => "authentication_error",
            Self::PermissionDenied(_) => "permission_error",
            Self::ServiceUnavailable(_) => "service_unavailable",
            Self::Internal(_) => "internal_error",
            Self::Injected { error_type, .. } => match error_type {
                InjectedErrorType::InvalidRequest => "invalid_request_error",
                InjectedErrorType::AuthenticationError => "authentication_error",
                _ => "api_error",
            },
            Self::SessionNotFound(_) => "not_found_error",
            Self::ContextLengthExceeded { .. } => "context_length_exceeded",
            Self::StreamError(_) => "stream_error",
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        let mut response = ErrorResponse::new(self.error_type(), &self.to_string());

        if let Self::Validation { param, .. } = self {
            if let Some(p) = param {
                response = response.with_param(p);
            }
        }

        if let Self::RateLimitExceeded { .. } = self {
            response = response.with_code("rate_limit_exceeded");
        }

        response
    }
}

impl IntoResponse for SimulationError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self.to_error_response());

        let mut response = (status, body).into_response();

        // Add retry-after header for rate limits
        if let SimulationError::RateLimitExceeded { retry_after } = &self {
            response.headers_mut().insert(
                "retry-after",
                retry_after.as_secs().to_string().parse().unwrap(),
            );
        }

        response
    }
}

impl From<std::io::Error> for SimulationError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for SimulationError {
    fn from(err: serde_json::Error) -> Self {
        Self::Validation {
            message: err.to_string(),
            param: None,
        }
    }
}

impl From<config::ConfigError> for SimulationError {
    fn from(err: config::ConfigError) -> Self {
        Self::Config(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            SimulationError::Validation {
                message: "test".into(),
                param: None
            }
            .status_code(),
            StatusCode::BAD_REQUEST
        );

        assert_eq!(
            SimulationError::RateLimitExceeded {
                retry_after: Duration::from_secs(60)
            }
            .status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );

        assert_eq!(
            SimulationError::ModelNotFound("gpt-4".into()).status_code(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse::new("invalid_request_error", "Invalid model")
            .with_param("model")
            .with_code("model_not_found");

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("invalid_request_error"));
        assert!(json.contains("Invalid model"));
    }
}
