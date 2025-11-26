# LLM-Simulator

Enterprise-grade offline LLM API simulator for testing and development.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-LLM--DevOps-blue.svg)](LICENSE)

## Overview

LLM-Simulator provides a drop-in replacement for production LLM APIs, enabling cost-effective, deterministic, and comprehensive testing of LLM-powered applications. It simulates OpenAI, Anthropic, and Google Gemini APIs with realistic latency, streaming support, and chaos engineering capabilities.

## Features

### Multi-Provider API Support
- **OpenAI** - Chat completions, embeddings, models endpoints (`/v1/chat/completions`, `/v1/embeddings`, `/v1/models`)
- **Anthropic** - Messages API (`/v1/messages`)
- **Google Gemini** - Generate content API (`/v1/models/{model}:generateContent`)

### Realistic Simulation
- **Latency Modeling** - Statistical distributions (log-normal, exponential, Pareto) for TTFT and ITL
- **Token-by-Token Streaming** - Server-Sent Events (SSE) with realistic inter-token delays
- **Deterministic Mode** - Seed-based RNG for reproducible tests

### Chaos Engineering
- **Error Injection** - Configurable error rates and types (rate limits, timeouts, server errors)
- **Circuit Breaker** - Simulate service degradation and recovery
- **Model-Specific Rules** - Target chaos to specific models or endpoints

### Enterprise Security
- **API Key Authentication** - Role-based access control (admin, user, readonly)
- **Rate Limiting** - Token bucket algorithm with configurable tiers
- **CORS Support** - Configurable origins and headers
- **Security Headers** - Production-ready security header configuration

### Observability
- **OpenTelemetry Integration** - Distributed tracing with OTLP export
- **Prometheus Metrics** - Request counts, latencies, error rates
- **Structured Logging** - JSON log format with trace correlation
- **Health Endpoints** - Liveness (`/health`) and readiness (`/ready`) probes

### High Performance
- **10,000+ RPS** - Optimized async architecture
- **<5ms Overhead** - Minimal latency impact
- **Graceful Shutdown** - Connection draining support

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/llm-devops/llm-simulator.git
cd llm-simulator

# Build release binary
cargo build --release

# Binary will be at ./target/release/llm-simulator
```

### Requirements
- Rust 1.75 or later
- Linux, macOS, or Windows

## Quick Start

### Start the Server

```bash
# Start with default settings
llm-simulator serve

# Start with custom port and chaos enabled
llm-simulator serve --port 9090 --chaos --chaos-probability 0.1

# Start with authentication
llm-simulator serve --require-auth --api-key "sk-test-key"

# Start with deterministic responses
llm-simulator serve --seed 42
```

### Send Requests

```bash
# OpenAI-compatible chat completion
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Anthropic-compatible messages
curl http://localhost:8080/v1/messages \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 256,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Streaming response
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

## CLI Reference

### Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `serve` | `s` | Start the simulator server |
| `generate` | `gen` | Generate test data or responses |
| `config` | `cfg` | Configuration management |
| `health` | - | Health check a running instance |
| `models` | - | Show available models |
| `benchmark` | `bench` | Benchmark the simulator |
| `client` | - | Send requests to a running instance |
| `version` | - | Show version and build information |

### Serve Command Options

```bash
llm-simulator serve [OPTIONS]

Options:
  -p, --port <PORT>              Port to listen on [default: 8080]
      --host <HOST>              Host to bind to [default: 0.0.0.0]
      --chaos                    Enable chaos engineering
      --chaos-probability <P>    Chaos probability (0.0-1.0)
      --no-latency               Disable latency simulation
      --latency-multiplier <M>   Latency multiplier (1.0 = normal)
      --seed <SEED>              Fixed seed for deterministic behavior
      --require-auth             Enable API key authentication
      --api-key <KEY>            API key for authentication
      --max-concurrent <N>       Maximum concurrent requests
      --timeout <SECONDS>        Request timeout
      --otlp-endpoint <URL>      OpenTelemetry endpoint
      --workers <N>              Worker threads (default: CPU count)
```

### Generate Command

```bash
# Generate a chat completion
llm-simulator generate chat --model gpt-4 --message "Hello" --format json

# Generate embeddings
llm-simulator generate embedding --text "Hello world" --dimensions 1536

# Generate sample configuration
llm-simulator generate config --format yaml --full

# Generate sample requests for testing
llm-simulator generate requests --count 100 --provider openai
```

### Client Command

```bash
# Send a chat request
llm-simulator client chat --url http://localhost:8080 --model gpt-4 "Hello!"

# Interactive chat session
llm-simulator client interactive --model gpt-4 --system "You are helpful"

# Generate embeddings
llm-simulator client embed --text "Hello world"
```

### Health Check

```bash
# Single health check
llm-simulator health --url http://localhost:8080

# Watch mode with 5-second interval
llm-simulator health --url http://localhost:8080 --watch --interval 5

# Check readiness
llm-simulator health --url http://localhost:8080 --ready
```

### Benchmark

```bash
# Basic benchmark
llm-simulator benchmark --url http://localhost:8080 --requests 1000

# High concurrency benchmark
llm-simulator benchmark --requests 10000 --concurrency 100 --model gpt-4

# Duration-based benchmark
llm-simulator benchmark --duration 60 --concurrency 50
```

## SDK Usage

The project includes a Rust SDK for programmatic access:

```rust
use llm_simulator::sdk::{Client, Provider};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a client
    let client = Client::builder()
        .base_url("http://localhost:8080")
        .api_key("sk-test-key")
        .default_model("gpt-4")
        .timeout(std::time::Duration::from_secs(30))
        .max_retries(3)
        .build()?;

    // Send a chat completion request
    let response = client
        .chat()
        .model("gpt-4")
        .system("You are a helpful assistant.")
        .message("What is the capital of France?")
        .temperature(0.7)
        .max_tokens(100)
        .send()
        .await?;

    println!("Response: {}", response.content());
    println!("Tokens used: {}", response.total_tokens());

    Ok(())
}
```

### Streaming

```rust
use futures::StreamExt;
use llm_simulator::sdk::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new("http://localhost:8080")?;

    let mut stream = client
        .stream()
        .model("gpt-4")
        .message("Tell me a story")
        .start()
        .await?;

    while let Some(chunk) = stream.next().await {
        if let Ok(c) = chunk {
            print!("{}", c.content);
        }
    }

    Ok(())
}
```

### Embeddings

```rust
use llm_simulator::sdk::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new("http://localhost:8080")?;

    let result = client
        .embeddings()
        .model("text-embedding-3-small")
        .input("Hello, world!")
        .dimensions(1536)
        .send()
        .await?;

    println!("Embedding dimensions: {}", result.dimensions());
    println!("Tokens used: {}", result.total_tokens());

    Ok(())
}
```

## Configuration

### Configuration File

Create `llm-simulator.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  max_concurrent_requests: 10000
  request_timeout: 300s
  cors_enabled: true
  cors_origins: ["*"]

latency:
  enabled: true
  multiplier: 1.0
  profiles:
    default:
      ttft:
        distribution: log_normal
        mean_ms: 200
        std_dev_ms: 50
      itl:
        distribution: exponential
        mean_ms: 30

chaos:
  enabled: false
  default_probability: 0.0
  rules: []

security:
  api_keys:
    enabled: false
    keys: []
  rate_limiting:
    enabled: true
    default_tier: standard
  cors:
    enabled: true
    allowed_origins: ["*"]

telemetry:
  enabled: true
  log_level: info
  json_logs: false
  trace_requests: true
  metrics_path: /metrics

default_provider: openai
seed: null  # Set for deterministic behavior
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LLM_SIMULATOR_PORT` | Server port | `8080` |
| `LLM_SIMULATOR_HOST` | Server host | `0.0.0.0` |
| `LLM_SIMULATOR_CONFIG` | Config file path | - |
| `LLM_SIMULATOR_SEED` | Random seed | - |
| `LLM_SIMULATOR_CHAOS` | Enable chaos | `false` |
| `LLM_SIMULATOR_NO_LATENCY` | Disable latency | `false` |
| `LLM_SIMULATOR_LOG_LEVEL` | Log level | `info` |
| `LLM_SIMULATOR_JSON_LOGS` | JSON log format | `false` |
| `LLM_SIMULATOR_REQUIRE_AUTH` | Require auth | `false` |
| `LLM_SIMULATOR_API_KEY` | API key | - |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP endpoint | - |

## Supported Models

### OpenAI Models
- `gpt-4`, `gpt-4-turbo`, `gpt-4o`, `gpt-4o-mini`
- `gpt-3.5-turbo`
- `text-embedding-ada-002`, `text-embedding-3-small`, `text-embedding-3-large`

### Anthropic Models
- `claude-3-5-sonnet-20241022`
- `claude-3-opus-20240229`
- `claude-3-sonnet-20240229`
- `claude-3-haiku-20240307`

### Google Models
- `gemini-1.5-pro`
- `gemini-1.5-flash`

## API Endpoints

### OpenAI-Compatible
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/chat/completions` | POST | Chat completions |
| `/v1/embeddings` | POST | Generate embeddings |
| `/v1/models` | GET | List models |
| `/v1/models/{id}` | GET | Get model details |

### Anthropic-Compatible
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/messages` | POST | Messages API |

### Google-Compatible
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/models/{model}:generateContent` | POST | Generate content |
| `/v1beta/models/{model}:generateContent` | POST | Beta endpoint |

### Health & Metrics
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Liveness check |
| `/ready` | GET | Readiness check |
| `/metrics` | GET | Prometheus metrics |
| `/version` | GET | Version info |

### Admin
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/admin/config` | GET | Current config |
| `/admin/stats` | GET | Runtime statistics |
| `/admin/chaos` | GET/POST | Chaos status |

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test suite
cargo test --test integration_tests
cargo test --test property_tests

# Run benchmarks
cargo bench
```

## Performance

Benchmark results on a typical development machine:

| Metric | Value |
|--------|-------|
| Throughput | 15,000+ RPS |
| P50 Latency | 0.8ms |
| P99 Latency | 3.2ms |
| Memory Usage | ~50MB base |

## License

This project is licensed under the LLM DevOps Permanent Source-Available License. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting pull requests.

## Support

- GitHub Issues: [Report bugs or request features](https://github.com/llm-devops/llm-simulator/issues)
- Documentation: See `/docs` directory for detailed guides
