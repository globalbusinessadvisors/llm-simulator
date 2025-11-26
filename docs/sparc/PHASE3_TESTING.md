# SPARC Specification: Phase 3 - Testing

## S - Specification

### Overview
Implement comprehensive testing infrastructure to achieve production-grade reliability through integration tests, property-based testing, and improved coverage across all modules.

### Objectives
1. Create integration test suite (minimum 30 tests)
2. Implement property-based tests with proptest
3. Add mock-based failure scenario tests
4. Complete streaming edge case coverage
5. Achieve 70% code coverage minimum

### Requirements

#### 3.1 Integration Tests
- **MUST** test full HTTP request/response cycles
- **MUST** cover all three providers (OpenAI, Anthropic, Google)
- **MUST** test streaming endpoints end-to-end
- **MUST** test error responses match API specs
- **SHOULD** test concurrent request handling
- **SHOULD** test configuration hot-reload scenarios

#### 3.2 Property-Based Tests
- **MUST** test latency distribution statistical properties
- **MUST** test request/response serialization roundtrips
- **MUST** test token estimation accuracy
- **SHOULD** test configuration validation exhaustively
- **SHOULD** test deterministic behavior with seeds

#### 3.3 Failure Scenario Tests
- **MUST** test rate limiting behavior
- **MUST** test circuit breaker state transitions
- **MUST** test chaos injection scenarios
- **SHOULD** test graceful degradation under load
- **SHOULD** test recovery after failures

#### 3.4 Streaming Tests
- **MUST** test stream completion with all providers
- **MUST** test stream interruption handling
- **MUST** test SSE format correctness
- **SHOULD** test large token count streaming
- **SHOULD** test keep-alive behavior

#### 3.5 Coverage Requirements
- **MUST** achieve 70% overall line coverage
- **MUST** achieve 80% coverage on critical paths (engine, handlers)
- **SHOULD** achieve 90% coverage on security module
- **MUST** fail CI if coverage drops below threshold

### Success Criteria
- Integration test directory populated with 30+ tests
- Property tests validate statistical properties
- All failure scenarios have test coverage
- Coverage gate enforced in CI
- No regressions in existing tests

---

## P - Pseudocode

### 3.1 Integration Test Framework

```
// tests/integration/common/mod.rs
MODULE test_common:
    STRUCT TestServer:
        addr: SocketAddr
        client: reqwest::Client
        shutdown_tx: oneshot::Sender<()>

    FUNCTION spawn_test_server(config: Option<SimulatorConfig>) -> TestServer:
        config = config.unwrap_or(SimulatorConfig::default())

        // Find available port
        port = find_available_port()
        config.server.port = port

        // Create shutdown channel
        (shutdown_tx, shutdown_rx) = oneshot::channel()

        // Spawn server in background
        tokio::spawn(async move {
            run_server_with_shutdown(config, shutdown_rx).await
        })

        // Wait for server to be ready
        wait_for_health(format!("http://127.0.0.1:{}/health", port))

        RETURN TestServer {
            addr: format!("127.0.0.1:{}", port).parse(),
            client: reqwest::Client::new(),
            shutdown_tx,
        }

    IMPL TestServer:
        FUNCTION post_json<T, R>(path: &str, body: &T) -> Result<R>:
            response = self.client
                .post(format!("http://{}{}", self.addr, path))
                .json(body)
                .send()
                .await?

            response.json().await

        FUNCTION shutdown(self):
            self.shutdown_tx.send(())
```

### 3.2 OpenAI Integration Tests

```
// tests/integration/openai_test.rs

#[tokio::test]
async fn test_chat_completion_basic():
    server = spawn_test_server(None)

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello")],
        ..Default::default()
    }

    response: ChatCompletionResponse = server
        .post_json("/v1/chat/completions", &request)
        .await
        .expect("Request should succeed")

    // Verify response structure
    assert!(response.id.starts_with("chatcmpl-"))
    assert_eq!(response.object, "chat.completion")
    assert_eq!(response.model, "gpt-4")
    assert!(!response.choices.is_empty())
    assert!(response.usage.total_tokens > 0)

    server.shutdown()

#[tokio::test]
async fn test_chat_completion_streaming():
    server = spawn_test_server(None)

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Count to 5")],
        stream: Some(true),
        ..Default::default()
    }

    response = server.client
        .post(format!("http://{}/v1/chat/completions", server.addr))
        .json(&request)
        .send()
        .await?

    // Collect SSE events
    events = collect_sse_events(response.bytes_stream()).await

    // Verify stream structure
    assert!(events.len() > 1, "Should have multiple chunks")
    assert!(events.iter().any(|e| e.contains("[DONE]")))

    // Verify each chunk is valid JSON
    for event in events.filter(|e| !e.contains("[DONE]")):
        chunk: ChatCompletionChunk = serde_json::from_str(event)?
        assert!(chunk.id.starts_with("chatcmpl-"))

    server.shutdown()

#[tokio::test]
async fn test_chat_completion_invalid_model():
    server = spawn_test_server(None)

    request = ChatCompletionRequest {
        model: "nonexistent-model".to_string(),
        messages: vec![Message::user("Hello")],
        ..Default::default()
    }

    response = server.client
        .post(format!("http://{}/v1/chat/completions", server.addr))
        .json(&request)
        .send()
        .await?

    assert_eq!(response.status(), 404)

    error: ErrorResponse = response.json().await?
    assert_eq!(error.error.type, "not_found_error")

    server.shutdown()

#[tokio::test]
async fn test_embeddings_basic():
    server = spawn_test_server(None)

    request = EmbeddingsRequest {
        model: "text-embedding-ada-002".to_string(),
        input: EmbeddingInput::Single("Hello world".to_string()),
        ..Default::default()
    }

    response: EmbeddingsResponse = server
        .post_json("/v1/embeddings", &request)
        .await?

    assert_eq!(response.object, "list")
    assert_eq!(response.data.len(), 1)
    assert_eq!(response.data[0].embedding.len(), 1536)  // ada-002 dimensions

    server.shutdown()

#[tokio::test]
async fn test_models_list():
    server = spawn_test_server(None)

    response: ModelsResponse = server
        .get_json("/v1/models")
        .await?

    assert_eq!(response.object, "list")
    assert!(response.data.len() >= 10, "Should have default models")

    // Verify known models exist
    model_ids: Vec<_> = response.data.iter().map(|m| &m.id).collect()
    assert!(model_ids.contains(&&"gpt-4".to_string()))
    assert!(model_ids.contains(&&"claude-3-5-sonnet-20241022".to_string()))

    server.shutdown()
```

### 3.3 Anthropic Integration Tests

```
// tests/integration/anthropic_test.rs

#[tokio::test]
async fn test_anthropic_messages_basic():
    server = spawn_test_server(None)

    request = AnthropicMessagesRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message::user("Hello")],
        max_tokens: 100,
        ..Default::default()
    }

    response: AnthropicMessagesResponse = server
        .post_json("/v1/messages", &request)
        .await?

    assert!(response.id.starts_with("msg_"))
    assert_eq!(response.type_field, "message")
    assert_eq!(response.role, "assistant")
    assert!(!response.content.is_empty())

    server.shutdown()

#[tokio::test]
async fn test_anthropic_streaming_events():
    server = spawn_test_server(None)

    request = AnthropicMessagesRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message::user("Hello")],
        max_tokens: 100,
        stream: Some(true),
        ..Default::default()
    }

    events = collect_anthropic_sse_events(
        server.post_stream("/v1/messages", &request).await
    )

    // Verify event sequence
    event_types: Vec<_> = events.iter().map(|e| &e.type_field).collect()

    assert!(event_types.contains(&"message_start"))
    assert!(event_types.contains(&"content_block_start"))
    assert!(event_types.contains(&"content_block_delta"))
    assert!(event_types.contains(&"content_block_stop"))
    assert!(event_types.contains(&"message_stop"))

    server.shutdown()
```

### 3.4 Property-Based Tests

```
// tests/property/latency_test.rs
use proptest::prelude::*;

proptest! {
    // Test that normal distribution produces values within expected range
    #[test]
    fn test_normal_distribution_bounds(
        mean in 10.0..1000.0f64,
        std_dev in 1.0..100.0f64,
        samples in 100..1000usize,
    ) {
        let config = LatencyDistribution::Normal { mean_ms: mean, std_dev_ms: std_dev };
        let sampler = DistributionSampler::new(&config, None);

        let values: Vec<f64> = (0..samples).map(|_| sampler.sample()).collect();

        // Statistical validation
        let actual_mean = values.iter().sum::<f64>() / values.len() as f64;
        let actual_std = calculate_std_dev(&values, actual_mean);

        // Mean should be within 20% of configured (with enough samples)
        prop_assert!((actual_mean - mean).abs() / mean < 0.2);

        // All values should be positive
        prop_assert!(values.iter().all(|&v| v >= 0.0));
    }

    // Test serialization roundtrip
    #[test]
    fn test_chat_request_roundtrip(
        model in "[a-z]{3,10}-[0-9]",
        message_count in 1..10usize,
        temperature in prop::option::of(0.0..2.0f32),
    ) {
        let messages: Vec<Message> = (0..message_count)
            .map(|i| Message::user(format!("Message {}", i)))
            .collect();

        let request = ChatCompletionRequest {
            model,
            messages,
            temperature,
            ..Default::default()
        };

        // Serialize to JSON
        let json = serde_json::to_string(&request).unwrap();

        // Deserialize back
        let parsed: ChatCompletionRequest = serde_json::from_str(&json).unwrap();

        // Verify equality
        prop_assert_eq!(request.model, parsed.model);
        prop_assert_eq!(request.messages.len(), parsed.messages.len());
        prop_assert_eq!(request.temperature, parsed.temperature);
    }

    // Test token estimation consistency
    #[test]
    fn test_token_estimation_proportional(
        word_count in 1..1000usize,
    ) {
        let text: String = (0..word_count)
            .map(|_| "word")
            .collect::<Vec<_>>()
            .join(" ");

        let tokens = estimate_tokens(&text);

        // Tokens should be roughly proportional to words
        // (approximately 0.75 tokens per word for English)
        let expected_min = (word_count as f64 * 0.5) as usize;
        let expected_max = (word_count as f64 * 1.5) as usize;

        prop_assert!(tokens >= expected_min);
        prop_assert!(tokens <= expected_max);
    }

    // Test deterministic behavior with seed
    #[test]
    fn test_deterministic_generation(seed: u64) {
        let config = SimulatorConfig {
            seed: Some(seed),
            ..Default::default()
        };

        let engine1 = SimulationEngine::new(config.clone());
        let engine2 = SimulationEngine::new(config);

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("Hello")],
            ..Default::default()
        };

        let response1 = engine1.chat_completion(&request).unwrap();
        let response2 = engine2.chat_completion(&request).unwrap();

        // Same seed should produce same output
        prop_assert_eq!(
            response1.choices[0].message.content,
            response2.choices[0].message.content
        );
    }
}
```

### 3.5 Failure Scenario Tests

```
// tests/integration/failure_test.rs

#[tokio::test]
async fn test_rate_limiting():
    config = SimulatorConfig {
        security: SecurityConfig {
            rate_limiting: RateLimitConfig {
                enabled: true,
                tiers: hashmap! {
                    "default" => RateLimitTier {
                        requests_per_minute: 5,
                        burst_size: 2,
                    }
                },
            },
            ..Default::default()
        },
        ..Default::default()
    }

    server = spawn_test_server(Some(config))

    // Make requests until rate limited
    success_count = 0
    rate_limited = false

    for _ in 0..20:
        response = server.get("/v1/models").await

        if response.status() == 429:
            rate_limited = true
            // Verify retry-after header
            assert!(response.headers().contains_key("retry-after"))
            break
        else:
            success_count += 1

    assert!(rate_limited, "Should have been rate limited")
    assert!(success_count >= 2, "Burst should allow some requests")
    assert!(success_count <= 7, "Should not exceed limit")

    server.shutdown()

#[tokio::test]
async fn test_circuit_breaker_opens():
    config = SimulatorConfig {
        chaos: ChaosConfig {
            enabled: true,
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 3,
                ..Default::default()
            },
            errors: vec![
                ErrorInjectionRule {
                    name: "always_fail".to_string(),
                    error_type: InjectedErrorType::ServerError,
                    probability: 1.0,
                    enabled: true,
                    ..Default::default()
                }
            ],
            ..Default::default()
        },
        ..Default::default()
    }

    server = spawn_test_server(Some(config))

    // Trigger failures to open circuit breaker
    for _ in 0..5:
        server.post_json::<_, Value>("/v1/chat/completions", &minimal_request()).await

    // Check circuit breaker status
    status: ChaosStatusResponse = server.get_json("/admin/chaos/status").await?

    assert!(status.circuit_breaker.is_open || status.circuit_breaker.half_open)

    server.shutdown()

#[tokio::test]
async fn test_chaos_error_injection():
    config = SimulatorConfig {
        chaos: ChaosConfig {
            enabled: true,
            global_probability: 1.0,
            errors: vec![
                ErrorInjectionRule {
                    name: "rate_limit".to_string(),
                    error_type: InjectedErrorType::RateLimit,
                    probability: 1.0,
                    models: vec!["gpt-4".to_string()],
                    enabled: true,
                    ..Default::default()
                }
            ],
            ..Default::default()
        },
        ..Default::default()
    }

    server = spawn_test_server(Some(config))

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello")],
        ..Default::default()
    }

    response = server.client
        .post(format!("http://{}/v1/chat/completions", server.addr))
        .json(&request)
        .send()
        .await?

    // Should get injected rate limit error
    assert_eq!(response.status(), 429)

    server.shutdown()
```

### 3.6 Streaming Edge Case Tests

```
// tests/integration/streaming_test.rs

#[tokio::test]
async fn test_stream_large_response():
    config = SimulatorConfig::default()
    config.models.get_mut("gpt-4").unwrap().generation.max_tokens = 4000

    server = spawn_test_server(Some(config))

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Write a long story")],
        stream: Some(true),
        max_tokens: Some(2000),
        ..Default::default()
    }

    events = collect_sse_events(
        server.post_stream("/v1/chat/completions", &request).await
    )

    // Should have many chunks for large response
    content_chunks = events.iter()
        .filter(|e| !e.contains("[DONE]") && !e.is_empty())
        .count()

    assert!(content_chunks > 10, "Large response should have many chunks")

    // Reconstruct full response
    full_content = reconstruct_streamed_content(&events)
    tokens = estimate_tokens(&full_content)

    assert!(tokens >= 1000, "Should have generated substantial content")

    server.shutdown()

#[tokio::test]
async fn test_stream_cancellation():
    server = spawn_test_server(None)

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Count to 1000")],
        stream: Some(true),
        ..Default::default()
    }

    // Start stream but cancel early
    response = server.client
        .post(format!("http://{}/v1/chat/completions", server.addr))
        .json(&request)
        .send()
        .await?

    let mut stream = response.bytes_stream()

    // Read only first few chunks
    for _ in 0..3:
        stream.next().await

    // Drop stream (cancellation)
    drop(stream)

    // Server should handle cancellation gracefully
    // Verify server is still healthy
    health = server.get_json::<HealthStatus>("/health").await?
    assert_eq!(health.status, "healthy")

    server.shutdown()

#[tokio::test]
async fn test_stream_keep_alive():
    server = spawn_test_server(None)

    // Configure slow response
    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello")],
        stream: Some(true),
        ..Default::default()
    }

    start = Instant::now()
    events = collect_sse_events_with_timeout(
        server.post_stream("/v1/chat/completions", &request).await,
        Duration::from_secs(30)
    )

    // If streaming took long enough, should have keep-alive
    if start.elapsed() > Duration::from_secs(15):
        // OpenAI uses comments for keep-alive
        has_keepalive = events.iter().any(|e| e.starts_with(":"))
        assert!(has_keepalive, "Long streams should have keep-alive")

    server.shutdown()

#[tokio::test]
async fn test_sse_format_correctness():
    server = spawn_test_server(None)

    request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello")],
        stream: Some(true),
        ..Default::default()
    }

    response = server.client
        .post(format!("http://{}/v1/chat/completions", server.addr))
        .json(&request)
        .send()
        .await?

    // Verify headers
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    )
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "no-cache"
    )

    // Verify SSE format
    raw_body = response.text().await?

    for line in raw_body.lines():
        if line.is_empty():
            continue  // Event separator
        if line.starts_with(":"):
            continue  // Comment/keep-alive
        if line.starts_with("data:"):
            data = line.strip_prefix("data:").trim()
            if data != "[DONE]":
                // Should be valid JSON
                serde_json::from_str::<Value>(data)?
        else:
            panic!("Invalid SSE line format: {}", line)

    server.shutdown()
```

---

## A - Architecture

### Test Directory Structure

```
tests/
├── integration/
│   ├── mod.rs                  # Test module declarations
│   ├── common/
│   │   ├── mod.rs              # Shared test utilities
│   │   ├── server.rs           # Test server spawning
│   │   ├── assertions.rs       # Custom assertions
│   │   └── fixtures.rs         # Request/response fixtures
│   ├── openai_test.rs          # OpenAI endpoint tests
│   ├── anthropic_test.rs       # Anthropic endpoint tests
│   ├── google_test.rs          # Google/Gemini endpoint tests
│   ├── streaming_test.rs       # SSE streaming tests
│   ├── failure_test.rs         # Error and chaos tests
│   ├── concurrent_test.rs      # Concurrency tests
│   └── config_test.rs          # Configuration tests
├── property/
│   ├── mod.rs
│   ├── latency_test.rs         # Latency distribution properties
│   ├── serialization_test.rs   # Roundtrip properties
│   └── determinism_test.rs     # Seed-based properties
└── fixtures/
    ├── requests/               # Sample request JSON files
    └── responses/              # Expected response JSON files
```

### Coverage Configuration

```toml
# .cargo/config.toml
[env]
CARGO_INCREMENTAL = "0"
RUSTFLAGS = "-Cinstrument-coverage"
LLVM_PROFILE_FILE = "coverage/cargo-test-%p-%m.profraw"

# codecov.yml
coverage:
  precision: 2
  round: down
  range: "70...100"
  status:
    project:
      default:
        target: 70%
        threshold: 2%
    patch:
      default:
        target: 80%
```

### CI Test Configuration

```yaml
# .github/workflows/ci.yml (test job)
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-action@stable

    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin

    - name: Run unit tests
      run: cargo test --lib --release

    - name: Run integration tests
      run: cargo test --test '*' --release

    - name: Run property tests
      run: cargo test --test property_* --release -- --test-threads=1

    - name: Generate coverage report
      run: |
        cargo tarpaulin \
          --out Xml \
          --out Html \
          --skip-clean \
          --timeout 300 \
          --ignore-tests

    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        files: cobertura.xml
        fail_ci_if_error: true
        threshold: 70

    - name: Fail if coverage below threshold
      run: |
        COVERAGE=$(grep -oP 'line-rate="\K[^"]+' cobertura.xml | head -1)
        PERCENTAGE=$(echo "$COVERAGE * 100" | bc)
        if (( $(echo "$PERCENTAGE < 70" | bc -l) )); then
          echo "Coverage $PERCENTAGE% is below 70% threshold"
          exit 1
        fi
```

---

## R - Refinement

### Edge Cases

1. **Concurrent Test Isolation**
   - Each test spawns its own server on unique port
   - Use `#[serial]` for tests that modify global state
   - Clean up resources in test teardown

2. **Flaky Test Prevention**
   - Use deterministic seeds where possible
   - Increase timeouts for CI environment
   - Retry flaky network operations

3. **Property Test Shrinking**
   - Configure proptest to shrink failing cases
   - Set reasonable bounds to prevent explosion
   - Save regression cases

4. **Test Parallelism**
   - Integration tests run in parallel by default
   - Property tests run single-threaded (--test-threads=1)
   - Use `cargo nextest` for better parallelism

### Error Handling in Tests

```rust
// Custom test assertions with better error messages
macro_rules! assert_json_eq {
    ($left:expr, $right:expr) => {
        let left_json: serde_json::Value = serde_json::from_str($left)
            .expect("Left side is not valid JSON");
        let right_json: serde_json::Value = serde_json::from_str($right)
            .expect("Right side is not valid JSON");

        assert_eq!(
            left_json, right_json,
            "JSON mismatch:\nLeft:  {}\nRight: {}",
            serde_json::to_string_pretty(&left_json).unwrap(),
            serde_json::to_string_pretty(&right_json).unwrap()
        );
    };
}

// Async test helper with timeout
async fn with_timeout<F, T>(future: F, duration: Duration) -> T
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .expect("Test timed out")
}
```

### Performance Testing

```rust
// Benchmark-style integration test
#[tokio::test]
async fn test_throughput_baseline():
    server = spawn_test_server(None)

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello")],
        ..Default::default()
    };

    let start = Instant::now();
    let request_count = 100;

    let handles: Vec<_> = (0..request_count)
        .map(|_| {
            let client = server.client.clone();
            let req = request.clone();
            tokio::spawn(async move {
                client.post(...).json(&req).send().await
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    let duration = start.elapsed();

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let rps = success_count as f64 / duration.as_secs_f64();

    println!("Throughput: {:.2} requests/second", rps);

    // Baseline assertion
    assert!(rps > 100.0, "Should handle >100 RPS");

    server.shutdown()
```

---

## C - Completion

### Definition of Done

- [ ] Integration test directory created with 30+ tests
- [ ] All three providers have endpoint tests
- [ ] Streaming tests cover all providers
- [ ] Property tests implemented with proptest
- [ ] Failure scenario tests for chaos/rate limiting
- [ ] Coverage report generated and uploaded
- [ ] Coverage gate of 70% enforced in CI
- [ ] All tests pass in CI environment
- [ ] Test documentation added to CONTRIBUTING.md

### Test Count Targets

| Category | Target | Minimum |
|----------|--------|---------|
| OpenAI endpoint tests | 10 | 8 |
| Anthropic endpoint tests | 6 | 4 |
| Google endpoint tests | 6 | 4 |
| Streaming tests | 8 | 6 |
| Failure scenario tests | 6 | 4 |
| Property tests | 10 | 6 |
| Configuration tests | 4 | 2 |
| **Total** | **50** | **34** |

### Verification Checklist

```bash
# 1. Run all tests
cargo test --all

# 2. Run integration tests only
cargo test --test '*'

# 3. Run property tests with verbose output
cargo test --test property_* -- --nocapture

# 4. Generate coverage report
cargo tarpaulin --out Html --open

# 5. Verify coverage threshold
cargo tarpaulin --fail-under 70

# 6. Run tests in CI mode
CI=true cargo test --release

# 7. Run specific test with output
cargo test test_chat_completion_streaming -- --nocapture

# 8. Run benchmarks (not in CI by default)
cargo bench
```

### Test Naming Convention

```
test_<module>_<scenario>_<expected_outcome>

Examples:
- test_openai_chat_completion_basic
- test_anthropic_streaming_events_sequence
- test_rate_limiting_enforced
- test_circuit_breaker_opens_after_failures
- test_deterministic_generation_with_seed
```

### Rollback Plan

1. Tests are additive - existing tests preserved
2. Coverage threshold can be adjusted in codecov.yml
3. Property tests can be disabled with `--skip property_`
4. Integration tests isolated from unit tests

### Monitoring Test Health

- CI job duration tracked (alert if >10 minutes)
- Flaky test detection (>2 failures in 10 runs)
- Coverage trend tracking in Codecov
- Test count tracked per release

