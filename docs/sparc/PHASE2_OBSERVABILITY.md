# SPARC Specification: Phase 2 - Observability

## S - Specification

### Overview
Enable comprehensive observability for the LLM-Simulator by activating distributed tracing, implementing meaningful health checks, and adding missing metrics to support production monitoring and alerting.

### Objectives
1. Enable OpenTelemetry distributed tracing
2. Implement comprehensive health check logic
3. Add missing metrics (queue_depth, provider/model labels)
4. Ensure alert rules match available metrics
5. Add trace correlation to logs

### Requirements

#### 2.1 Distributed Tracing
- **MUST** call `init_otel()` when OTLP endpoint is configured
- **MUST** create spans for each HTTP request
- **MUST** propagate trace context via W3C headers
- **MUST** add span attributes for model, provider, token counts
- **SHOULD** sample traces based on configuration (default 100%)
- **SHOULD** export to OTLP gRPC endpoint

#### 2.2 Health Check Logic
- **MUST** verify engine initialization state
- **MUST** check configuration validity
- **SHOULD** verify telemetry pipeline connectivity
- **SHOULD** report degraded state (not just healthy/unhealthy)
- **MUST** differentiate liveness vs readiness checks
- **MUST** return detailed component status in response

#### 2.3 Missing Metrics
- **MUST** add `llm_simulator_queue_depth` gauge
- **MUST** add `llm_simulator_queue_capacity` gauge
- **MUST** add provider label to all request metrics
- **MUST** add model label to all request metrics
- **SHOULD** add `llm_simulator_cost_dollars` counter
- **SHOULD** add `llm_simulator_cache_hits_total` counter

#### 2.4 Log-Trace Correlation
- **MUST** include `trace_id` in all log entries when tracing enabled
- **MUST** include `span_id` in all log entries when tracing enabled
- **SHOULD** use structured logging format for trace context
- **SHOULD** support log sampling based on trace sampling

### Success Criteria
- Traces visible in Jaeger/OTLP backend
- Health endpoint returns actual system state
- All alert rules reference existing metrics
- Logs correlate with traces via trace_id
- 100% of requests have trace context

---

## P - Pseudocode

### 2.1 Enable Distributed Tracing

```
FUNCTION run_server(config):
    // Initialize telemetry first
    init_telemetry(config.telemetry)

    // NEW: Initialize OpenTelemetry if endpoint configured
    IF config.telemetry.otlp_endpoint IS NOT NULL:
        init_otel(config.telemetry)
        log.info("OpenTelemetry tracing enabled", endpoint=config.telemetry.otlp_endpoint)

    // Build router with tracing middleware
    router = build_router(config)
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    trace_id = tracing::field::Empty,
                    span_id = tracing::field::Empty,
                )
            })
            .on_response(|response, latency, span| {
                span.record("status", response.status().as_u16());
                span.record("latency_ms", latency.as_millis());
            })
        )

    // Start server
    serve(router, config.server.socket_addr())
```

### 2.2 Request Tracing Instrumentation

```
FUNCTION handle_chat_completion(request, state):
    // Extract or create trace context
    trace_context = extract_w3c_trace_context(request.headers)

    // Create span for this request
    span = tracing::info_span!(
        "chat_completion",
        model = request.model,
        provider = determine_provider(request.model),
        input_tokens = tracing::field::Empty,
        output_tokens = tracing::field::Empty,
        stream = request.stream,
    )

    // Execute within span context
    WITH span.entered():
        // Record input tokens
        input_tokens = estimate_tokens(request.messages)
        span.record("input_tokens", input_tokens)

        // Generate response
        response = state.engine.chat_completion(request)

        // Record output tokens
        span.record("output_tokens", response.usage.completion_tokens)

        // Add trace context to response headers
        inject_w3c_trace_context(response.headers, span.context())

        RETURN response
```

### 2.3 Health Check Implementation

```
STRUCT HealthStatus:
    status: "healthy" | "degraded" | "unhealthy"
    version: String
    uptime_seconds: u64
    checks: Map<String, ComponentHealth>

STRUCT ComponentHealth:
    status: "pass" | "warn" | "fail"
    message: Option<String>
    latency_ms: Option<u64>

FUNCTION health_check(state) -> HealthStatus:
    checks = Map::new()
    overall_status = "healthy"

    // Check 1: Engine initialization
    engine_check = check_engine(state.engine)
    checks.insert("engine", engine_check)
    IF engine_check.status == "fail":
        overall_status = "unhealthy"

    // Check 2: Configuration validity
    config_check = check_config(state.config)
    checks.insert("config", config_check)
    IF config_check.status == "fail":
        overall_status = "unhealthy"

    // Check 3: Metrics subsystem
    metrics_check = check_metrics(state.metrics)
    checks.insert("metrics", metrics_check)
    IF metrics_check.status == "warn":
        IF overall_status == "healthy":
            overall_status = "degraded"

    // Check 4: Memory usage
    memory_check = check_memory()
    checks.insert("memory", memory_check)
    IF memory_check.status == "warn":
        IF overall_status == "healthy":
            overall_status = "degraded"

    RETURN HealthStatus {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        checks: checks,
    }

FUNCTION check_engine(engine) -> ComponentHealth:
    TRY:
        // Verify engine can process a minimal request
        start = Instant::now()
        models = engine.list_models()
        latency = start.elapsed()

        IF models.is_empty():
            RETURN ComponentHealth {
                status: "warn",
                message: Some("No models configured"),
                latency_ms: Some(latency.as_millis()),
            }

        RETURN ComponentHealth {
            status: "pass",
            message: None,
            latency_ms: Some(latency.as_millis()),
        }
    CATCH error:
        RETURN ComponentHealth {
            status: "fail",
            message: Some(error.to_string()),
            latency_ms: None,
        }

FUNCTION readiness_check(state) -> bool:
    health = health_check(state)
    // Ready only if healthy or degraded (not unhealthy)
    RETURN health.status != "unhealthy"
```

### 2.4 Missing Metrics Implementation

```
STRUCT EnhancedMetrics:
    // Existing metrics
    requests_total: Counter
    request_duration: Histogram
    tokens_input: Counter
    tokens_output: Counter
    errors_total: Counter
    active_requests: Gauge

    // NEW: Queue metrics
    queue_depth: Gauge
    queue_capacity: Gauge

    // NEW: Cost tracking
    cost_dollars: Counter

    // NEW: Cache metrics
    cache_hits: Counter
    cache_misses: Counter

FUNCTION record_request(metrics, request, response, duration):
    // Get labels
    provider = determine_provider(request.model)
    model = request.model

    // Record with labels
    metrics.requests_total
        .with_label("provider", provider)
        .with_label("model", model)
        .with_label("status", response.status)
        .increment()

    metrics.request_duration
        .with_label("provider", provider)
        .with_label("model", model)
        .record(duration.as_secs_f64())

    metrics.tokens_input
        .with_label("provider", provider)
        .with_label("model", model)
        .increment_by(response.usage.prompt_tokens)

    metrics.tokens_output
        .with_label("provider", provider)
        .with_label("model", model)
        .increment_by(response.usage.completion_tokens)

    // Calculate and record cost
    cost = calculate_cost(provider, model, response.usage)
    metrics.cost_dollars
        .with_label("provider", provider)
        .with_label("model", model)
        .increment_by(cost)

FUNCTION update_queue_metrics(metrics, queue):
    metrics.queue_depth.set(queue.len())
    metrics.queue_capacity.set(queue.capacity())
```

### 2.5 Log-Trace Correlation

```
FUNCTION init_telemetry_with_tracing(config):
    // Create base subscriber
    subscriber = tracing_subscriber::fmt()
        .with_env_filter(config.log_level)
        .with_target(true)
        .with_thread_ids(true)

    // Add JSON formatting if configured
    IF config.json_logs:
        subscriber = subscriber.json()

    // NEW: Add OpenTelemetry layer for trace context
    IF config.otlp_endpoint IS NOT NULL:
        otel_layer = tracing_opentelemetry::layer()
            .with_tracer(init_tracer(config))

        subscriber = subscriber
            .with(otel_layer)
            .with(TraceContextLayer::new())  // Injects trace_id into logs

    subscriber.init()

// Custom layer to inject trace context into log records
STRUCT TraceContextLayer

IMPL Layer FOR TraceContextLayer:
    FUNCTION on_event(event, context):
        // Get current span
        IF current_span = context.current_span():
            span_context = current_span.context()
            IF span_context.is_valid():
                // Add trace_id and span_id to event
                event.record("trace_id", span_context.trace_id().to_string())
                event.record("span_id", span_context.span_id().to_string())
```

---

## A - Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Observability Architecture                          │
└─────────────────────────────────────────────────────────────────────────┘

                              ┌─────────────────┐
                              │   OTLP Collector │
                              │   (Jaeger/Tempo) │
                              └────────▲────────┘
                                       │ gRPC/HTTP
                                       │
┌─────────────────────────────────────┴─────────────────────────────────┐
│                           LLM-Simulator                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Tracing   │  │   Metrics   │  │   Logging   │  │   Health    │  │
│  │   Layer     │  │   Registry  │  │  Subscriber │  │   Checker   │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
│         │                │                │                │          │
│         │    ┌───────────┴────────────────┴───────────┐   │          │
│         │    │         Request Handler                 │   │          │
│         └────┤  - Creates spans per request            │───┘          │
│              │  - Records metrics with labels          │              │
│              │  - Logs with trace context              │              │
│              │  - Reports component health             │              │
│              └─────────────────────────────────────────┘              │
└───────────────────────────────────────────────────────────────────────┘
         │                      │                      │
         ▼                      ▼                      ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│   Jaeger UI     │   │   Prometheus    │   │   Log Aggregator │
│   (Traces)      │   │   (Metrics)     │   │   (ELK/Loki)    │
└─────────────────┘   └─────────────────┘   └─────────────────┘
```

### File Structure

```
src/
├── telemetry/
│   ├── mod.rs              # Updated: call init_otel()
│   ├── metrics.rs          # Updated: add queue/cost/cache metrics
│   ├── tracing.rs          # NEW: span creation utilities
│   └── correlation.rs      # NEW: log-trace correlation layer
├── server/
│   ├── handlers.rs         # Updated: real health check logic
│   └── middleware.rs       # Updated: tracing instrumentation
└── engine/
    └── state.rs            # Updated: expose queue metrics
```

### Health Check Response Schema

```json
{
  "status": "healthy|degraded|unhealthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "timestamp": "2025-11-26T12:00:00Z",
  "checks": {
    "engine": {
      "status": "pass",
      "latency_ms": 2
    },
    "config": {
      "status": "pass"
    },
    "metrics": {
      "status": "pass"
    },
    "memory": {
      "status": "warn",
      "message": "Memory usage at 75%",
      "value": 75
    },
    "tracing": {
      "status": "pass",
      "message": "Connected to OTLP endpoint"
    }
  }
}
```

### Metrics Schema

```prometheus
# Existing metrics - now with labels
llm_simulator_requests_total{provider="openai",model="gpt-4",status="success"} 1234
llm_simulator_request_duration_seconds{provider="openai",model="gpt-4",quantile="0.99"} 2.5
llm_simulator_tokens_input_total{provider="openai",model="gpt-4"} 50000
llm_simulator_tokens_output_total{provider="openai",model="gpt-4"} 25000
llm_simulator_errors_total{provider="openai",model="gpt-4",error_type="rate_limit"} 10

# NEW: Queue metrics (referenced in alerts)
llm_simulator_queue_depth 42
llm_simulator_queue_capacity 1000

# NEW: Cost tracking
llm_simulator_cost_dollars_total{provider="openai",model="gpt-4"} 12.50

# NEW: Cache metrics
llm_simulator_cache_hits_total{cache="response"} 500
llm_simulator_cache_misses_total{cache="response"} 100
```

### Trace Context Propagation

```
Incoming Request Headers:
  traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
  tracestate: vendor=value

         │
         ▼

┌─────────────────────────────────────────────────────────────────────┐
│                        LLM-Simulator                                 │
│                                                                      │
│  Span: http_request                                                  │
│  ├── trace_id: 0af7651916cd43dd8448eb211c80319c                     │
│  ├── span_id: b7ad6b7169203331 (parent)                             │
│  └── new_span_id: c8be7c8270314442 (created)                        │
│                                                                      │
│  Child Spans:                                                        │
│  ├── chat_completion (model=gpt-4, provider=openai)                 │
│  │   ├── latency_simulation                                         │
│  │   └── response_generation                                        │
│  └── metrics_recording                                              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘

         │
         ▼

Outgoing Response Headers:
  traceparent: 00-0af7651916cd43dd8448eb211c80319c-c8be7c8270314442-01
```

---

## R - Refinement

### Edge Cases

1. **OTLP Endpoint Unavailable**
   - Continue operation without tracing (degraded mode)
   - Log warning, don't fail startup
   - Report in health check as degraded

2. **High Cardinality Labels**
   - Limit model label to known models
   - Use "unknown" for unrecognized models
   - Monitor label cardinality

3. **Memory Pressure from Trace Buffering**
   - Configure batch export with limits
   - Drop traces under memory pressure
   - Alert on trace drop rate

4. **Health Check During Startup**
   - Return 503 during initialization
   - Startup probe should tolerate initial failures
   - Track initialization progress

5. **Clock Drift in Distributed Tracing**
   - Use NTP-synchronized clocks
   - Tracing libraries handle minor drift
   - Document timing requirements

### Error Handling

```rust
// Graceful degradation for tracing failures
pub fn init_otel(config: &TelemetryConfig) -> Result<(), TracingError> {
    match setup_otlp_exporter(&config.otlp_endpoint) {
        Ok(exporter) => {
            // Install tracer
            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .install_batch(opentelemetry_sdk::runtime::Tokio)?;

            tracing::info!("OpenTelemetry tracing initialized");
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to initialize OpenTelemetry, tracing disabled"
            );
            // Don't fail - operate without tracing
            Ok(())
        }
    }
}

// Health check with graceful handling
pub async fn health_check(state: &AppState) -> HealthStatus {
    let mut checks = HashMap::new();

    // Each check wrapped in catch_unwind for resilience
    checks.insert("engine", catch_check(|| check_engine(&state.engine)));
    checks.insert("config", catch_check(|| check_config(&state.config)));
    checks.insert("metrics", catch_check(|| check_metrics(&state.metrics)));

    // Determine overall status
    let overall = if checks.values().any(|c| c.status == "fail") {
        "unhealthy"
    } else if checks.values().any(|c| c.status == "warn") {
        "degraded"
    } else {
        "healthy"
    };

    HealthStatus { status: overall, checks, .. }
}
```

### Performance Considerations

1. **Tracing Overhead**: ~1-5μs per span when enabled
2. **Metric Recording**: ~100ns per metric with labels
3. **Health Check**: Should complete in <100ms
4. **Log Correlation**: Minimal overhead with proper layer ordering

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    // Test tracing initialization
    #[test]
    fn test_otel_init_without_endpoint() {
        let config = TelemetryConfig { otlp_endpoint: None, .. };
        // Should not panic, tracing disabled
        assert!(init_telemetry(&config).is_ok());
    }

    // Test health check logic
    #[tokio::test]
    async fn test_health_check_healthy() {
        let state = create_test_state();
        let health = health_check(&state).await;
        assert_eq!(health.status, "healthy");
    }

    #[tokio::test]
    async fn test_health_check_degraded() {
        let state = create_test_state_with_warning();
        let health = health_check(&state).await;
        assert_eq!(health.status, "degraded");
    }

    // Test metrics with labels
    #[test]
    fn test_metrics_with_provider_label() {
        let metrics = SimulatorMetrics::new();
        metrics.record_request("openai", "gpt-4", Duration::from_millis(100));
        // Verify metric recorded with correct labels
    }

    // Test log-trace correlation
    #[test]
    fn test_log_contains_trace_id() {
        // Setup tracing with test exporter
        // Make request
        // Verify logs contain trace_id field
    }
}
```

---

## C - Completion

### Definition of Done

- [ ] `init_otel()` called in `run_server()` when endpoint configured
- [ ] Spans created for all HTTP request types
- [ ] W3C trace context propagation working
- [ ] Health check verifies engine, config, metrics status
- [ ] Readiness check returns false when unhealthy
- [ ] `queue_depth` and `queue_capacity` metrics implemented
- [ ] Provider and model labels on all request metrics
- [ ] Log entries include trace_id when tracing enabled
- [ ] All alert rules reference existing metrics
- [ ] Documentation updated with observability guide

### Verification Checklist

```bash
# 1. Verify tracing is enabled
curl -s http://localhost:8080/health | jq '.checks.tracing'
# Expected: {"status": "pass", "message": "Connected to OTLP endpoint"}

# 2. Make request and check Jaeger
curl -H "Authorization: Bearer $KEY" http://localhost:8080/v1/chat/completions -d '{...}'
# Check Jaeger UI for trace with model/provider attributes

# 3. Verify health check logic
# Stop a component and check health
curl -s http://localhost:8080/health | jq '.status'
# Expected: "degraded" or "unhealthy"

# 4. Verify new metrics
curl -s http://localhost:8080/metrics | grep queue_depth
# Expected: llm_simulator_queue_depth 0

curl -s http://localhost:8080/metrics | grep 'requests_total.*provider'
# Expected: llm_simulator_requests_total{provider="openai",model="gpt-4",...}

# 5. Verify log-trace correlation
# Check logs for trace_id field
docker logs llm-simulator 2>&1 | grep trace_id
# Expected: Logs with trace_id="..." field

# 6. Verify alerts can fire
# Prometheus query test
curl -s http://localhost:9090/api/v1/query?query=llm_simulator_queue_depth
# Expected: Result with current value
```

### Alert Rule Updates Required

```yaml
# deploy/prometheus/rules/alerts.yml
# Update to use correct metric names

# BEFORE (broken - metric doesn't exist)
- alert: HighQueueDepth
  expr: llm_simulator_queue_depth > 100

# AFTER (fixed - metric now exists)
- alert: HighQueueDepth
  expr: llm_simulator_queue_depth > 100
  for: 2m
  labels:
    severity: warning
  annotations:
    summary: "Queue depth is high"
    description: "Queue depth is {{ $value }} (threshold: 100)"
```

### Rollback Plan

1. Feature flag `OTEL_ENABLED=false` disables tracing
2. Health check always passes in `HEALTH_CHECK_SIMPLE=true` mode
3. Old metrics still available (new ones additive)
4. Log format unchanged if JSON disabled

### Monitoring the Observability System

- Alert: Trace export failures > 1% of traces
- Alert: Health check latency > 100ms
- Alert: Metric cardinality > 10,000 series
- Dashboard: Trace sampling rate, export success rate
- Dashboard: Health check response times

