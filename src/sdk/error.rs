//! SDK Error Types
//!
//! Error types for the LLM-Simulator SDK.

use std::fmt;

/// SDK Error type
#[derive(Debug)]
pub enum SdkError {
    /// HTTP request error
    Request(reqwest::Error),

    /// HTTP response error with status code
    Response {
        status: u16,
        message: String,
        body: Option<serde_json::Value>,
    },

    /// JSON serialization/deserialization error
    Json(serde_json::Error),

    /// URL parsing error
    Url(url::ParseError),

    /// Configuration error
    Config(String),

    /// Timeout error
    Timeout,

    /// Connection error
    Connection(String),

    /// Authentication error
    Authentication(String),

    /// Rate limit exceeded
    RateLimit {
        retry_after: Option<u64>,
    },

    /// Model not found
    ModelNotFound(String),

    /// Invalid request
    InvalidRequest(String),

    /// Stream error
    Stream(String),

    /// Retry exhausted
    RetryExhausted {
        attempts: u32,
        last_error: Box<SdkError>,
    },
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdkError::Request(e) => write!(f, "Request error: {}", e),
            SdkError::Response { status, message, .. } => {
                write!(f, "Response error ({}): {}", status, message)
            }
            SdkError::Json(e) => write!(f, "JSON error: {}", e),
            SdkError::Url(e) => write!(f, "URL error: {}", e),
            SdkError::Config(msg) => write!(f, "Configuration error: {}", msg),
            SdkError::Timeout => write!(f, "Request timeout"),
            SdkError::Connection(msg) => write!(f, "Connection error: {}", msg),
            SdkError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            SdkError::RateLimit { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "Rate limit exceeded, retry after {} seconds", secs)
                } else {
                    write!(f, "Rate limit exceeded")
                }
            }
            SdkError::ModelNotFound(model) => write!(f, "Model not found: {}", model),
            SdkError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            SdkError::Stream(msg) => write!(f, "Stream error: {}", msg),
            SdkError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {} attempts: {}", attempts, last_error)
            }
        }
    }
}

impl std::error::Error for SdkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SdkError::Request(e) => Some(e),
            SdkError::Json(e) => Some(e),
            SdkError::Url(e) => Some(e),
            SdkError::RetryExhausted { last_error, .. } => Some(last_error.as_ref()),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for SdkError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            SdkError::Timeout
        } else if e.is_connect() {
            SdkError::Connection(e.to_string())
        } else {
            SdkError::Request(e)
        }
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(e: serde_json::Error) -> Self {
        SdkError::Json(e)
    }
}

impl From<url::ParseError> for SdkError {
    fn from(e: url::ParseError) -> Self {
        SdkError::Url(e)
    }
}

/// SDK Result type
pub type SdkResult<T> = Result<T, SdkError>;

impl SdkError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            SdkError::Request(e) => e.is_timeout() || e.is_connect(),
            SdkError::Response { status, .. } => {
                // 5xx errors and 429 are retryable
                *status >= 500 || *status == 429
            }
            SdkError::Timeout => true,
            SdkError::Connection(_) => true,
            SdkError::RateLimit { .. } => true,
            _ => false,
        }
    }

    /// Get the retry-after duration if available
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            SdkError::RateLimit { retry_after } => {
                retry_after.map(std::time::Duration::from_secs)
            }
            _ => None,
        }
    }

    /// Create a response error from status and body
    pub fn from_response(status: u16, body: serde_json::Value) -> Self {
        let message = body["error"]["message"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .unwrap_or("Unknown error")
            .to_string();

        match status {
            401 => SdkError::Authentication(message),
            404 => {
                if let Some(model) = body["error"]["param"].as_str() {
                    SdkError::ModelNotFound(model.to_string())
                } else {
                    SdkError::Response { status, message, body: Some(body) }
                }
            }
            429 => {
                let retry_after = body["error"]["retry_after"]
                    .as_u64()
                    .or_else(|| body["retry_after"].as_u64());
                SdkError::RateLimit { retry_after }
            }
            _ => SdkError::Response { status, message, body: Some(body) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SdkError::ModelNotFound("gpt-5".to_string());
        assert!(err.to_string().contains("gpt-5"));

        let err = SdkError::Timeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_retryable() {
        assert!(SdkError::Timeout.is_retryable());
        assert!(SdkError::RateLimit { retry_after: Some(5) }.is_retryable());
        assert!(SdkError::Response { status: 503, message: "Unavailable".to_string(), body: None }.is_retryable());
        assert!(!SdkError::Authentication("bad key".to_string()).is_retryable());
    }

    #[test]
    fn test_from_response() {
        let body = serde_json::json!({
            "error": {
                "message": "Rate limit exceeded",
                "retry_after": 60
            }
        });

        let err = SdkError::from_response(429, body);
        assert!(matches!(err, SdkError::RateLimit { retry_after: Some(60) }));
    }
}
