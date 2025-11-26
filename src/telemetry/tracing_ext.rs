//! Tracing Extensions
//!
//! Provides utilities for distributed tracing:
//! - Span creation and context propagation
//! - Log-trace correlation
//! - Request/response tracking

use std::time::Instant;
use tracing::{info_span, Span};

/// Request span for distributed tracing
#[derive(Debug)]
pub struct RequestSpan {
    span: Span,
    start: Instant,
    request_id: String,
    trace_id: Option<String>,
    span_id: Option<String>,
}

impl RequestSpan {
    /// Create a new request span
    pub fn new(
        request_id: &str,
        method: &str,
        path: &str,
        model: Option<&str>,
        provider: Option<&str>,
    ) -> Self {
        let span = info_span!(
            "http_request",
            request_id = %request_id,
            method = %method,
            path = %path,
            model = model.unwrap_or("unknown"),
            provider = provider.unwrap_or("unknown"),
            trace_id = tracing::field::Empty,
            span_id = tracing::field::Empty,
            status = tracing::field::Empty,
            latency_ms = tracing::field::Empty,
            input_tokens = tracing::field::Empty,
            output_tokens = tracing::field::Empty,
        );

        // Extract trace context if available
        let (trace_id, span_id) = extract_trace_context();

        if let Some(ref tid) = trace_id {
            span.record("trace_id", tid.as_str());
        }
        if let Some(ref sid) = span_id {
            span.record("span_id", sid.as_str());
        }

        Self {
            span,
            start: Instant::now(),
            request_id: request_id.to_string(),
            trace_id,
            span_id,
        }
    }

    /// Get the underlying span
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Record response status
    pub fn record_status(&self, status: u16) {
        self.span.record("status", status);
    }

    /// Record token counts
    pub fn record_tokens(&self, input: u32, output: u32) {
        self.span.record("input_tokens", input);
        self.span.record("output_tokens", output);
    }

    /// Record latency
    pub fn record_latency(&self) {
        self.span.record("latency_ms", self.start.elapsed().as_millis() as u64);
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get trace ID if available
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    /// Get span ID if available
    pub fn span_id(&self) -> Option<&str> {
        self.span_id.as_deref()
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// Extract trace context from current span
/// Returns (trace_id, span_id) if available
fn extract_trace_context() -> (Option<String>, Option<String>) {
    // In a real OTEL implementation, we'd extract from the current span context
    // For now, we generate a pseudo trace ID based on the current time
    // This will be replaced with real OTEL context when enabled

    #[cfg(feature = "otel")]
    {
        use opentelemetry::trace::TraceContextExt;
        use tracing_opentelemetry::OpenTelemetrySpanExt;

        let context = tracing::Span::current().context();
        let span_ref = context.span();
        let span_context = span_ref.span_context();

        if span_context.is_valid() {
            return (
                Some(span_context.trace_id().to_string()),
                Some(span_context.span_id().to_string()),
            );
        }
    }

    // Fallback: no trace context available
    (None, None)
}

/// Log with trace context
#[macro_export]
macro_rules! trace_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(
            target: "llm_simulator",
            $($arg)*
        )
    };
}

/// Create a span for database/external operations
pub fn create_operation_span(operation: &str, target: &str) -> Span {
    info_span!(
        "operation",
        operation = %operation,
        target = %target,
        duration_ms = tracing::field::Empty,
        success = tracing::field::Empty,
    )
}

/// Create a span for streaming operations
pub fn create_stream_span(request_id: &str, model: &str) -> Span {
    info_span!(
        "stream",
        request_id = %request_id,
        model = %model,
        chunks_sent = tracing::field::Empty,
        total_tokens = tracing::field::Empty,
        ttft_ms = tracing::field::Empty,
    )
}

/// Trace context holder for log correlation
#[derive(Debug, Clone, Default)]
pub struct TraceContext {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    /// Create from current span
    pub fn current() -> Self {
        let (trace_id, span_id) = extract_trace_context();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
        }
    }

    /// Check if context is valid
    pub fn is_valid(&self) -> bool {
        self.trace_id.is_some() && self.span_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_span_creation() {
        let span = RequestSpan::new(
            "req-123",
            "POST",
            "/v1/chat/completions",
            Some("gpt-4"),
            Some("openai"),
        );

        assert_eq!(span.request_id(), "req-123");
    }

    #[test]
    fn test_request_span_recording() {
        let span = RequestSpan::new(
            "req-456",
            "POST",
            "/v1/chat/completions",
            Some("gpt-4"),
            None,
        );

        span.record_status(200);
        span.record_tokens(100, 50);
        span.record_latency();

        // Span should be recordable without panicking
    }

    #[test]
    fn test_trace_context() {
        let ctx = TraceContext::current();
        // Without OTEL, context should be empty
        assert!(!ctx.is_valid());
    }

    #[test]
    fn test_operation_span() {
        let _span = create_operation_span("query", "engine");
        // Should create without panicking
    }

    #[test]
    fn test_stream_span() {
        let _span = create_stream_span("req-789", "gpt-4");
        // Should create without panicking
    }
}
