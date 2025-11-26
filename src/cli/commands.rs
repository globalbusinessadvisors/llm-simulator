//! CLI Command Implementations
//!
//! Implementations for all CLI subcommands.

use std::io::{self, Write, BufRead};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Result, Context, bail};
use tokio::sync::Semaphore;

use crate::{SimulatorConfig, SimulationEngine, VERSION};
use crate::types::*;

use super::{
    Cli, Commands, ServeCommand, GenerateCommand, GenerateAction,
    ConfigCommand, ConfigAction, HealthCommand, ModelsCommand,
    BenchmarkCommand, ClientCommand, ClientAction,
};

/// Execute the CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    // Load base configuration
    let mut config = if let Some(path) = &cli.config {
        SimulatorConfig::from_file(path)?
    } else {
        SimulatorConfig::from_env()?
    };

    // Apply global settings
    config.telemetry.log_level = cli.log_level.clone();
    config.telemetry.json_logs = cli.json_logs;

    match cli.command {
        Commands::Serve(cmd) => execute_serve(cmd, config, cli.quiet).await,
        Commands::Generate(cmd) => execute_generate(cmd, config).await,
        Commands::Config(cmd) => execute_config(cmd, config, cli.config).await,
        Commands::Health(cmd) => execute_health(cmd).await,
        Commands::Models(cmd) => execute_models(cmd, config).await,
        Commands::Benchmark(cmd) => execute_benchmark(cmd).await,
        Commands::Client(cmd) => execute_client(cmd).await,
        Commands::Version => execute_version(),
    }
}

/// Execute the serve command
async fn execute_serve(cmd: ServeCommand, mut config: SimulatorConfig, quiet: bool) -> Result<()> {
    // Apply command overrides
    config.server.port = cmd.port;
    config.server.host = cmd.host;
    config.chaos.enabled = cmd.chaos;
    config.latency.enabled = !cmd.no_latency;
    config.telemetry.otlp_endpoint = cmd.otlp_endpoint;

    if let Some(prob) = cmd.chaos_probability {
        config.chaos.global_probability = prob;
    }

    if let Some(mult) = cmd.latency_multiplier {
        config.latency.multiplier = mult;
    }

    if let Some(seed) = cmd.seed {
        config.seed = Some(seed);
    }

    if let Some(max_concurrent) = cmd.max_concurrent {
        config.server.max_concurrent_requests = max_concurrent;
    }

    if let Some(timeout) = cmd.timeout {
        config.server.request_timeout_secs = timeout;
    }

    if cmd.require_auth {
        config.security.api_keys.enabled = true;
        if let Some(key) = cmd.api_key {
            use crate::config::security::{ApiKeyEntry, ApiKeyRole, RateLimitTier};
            config.security.api_keys.keys.push(ApiKeyEntry {
                id: "cli-generated".to_string(),
                key,
                role: ApiKeyRole::Admin,
                rate_limit_tier: RateLimitTier::Unlimited,
                description: Some("CLI Generated Key".to_string()),
                enabled: true,
            });
        }
    }

    // Validate configuration
    config.validate().context("Configuration validation failed")?;

    // Print startup banner
    if !quiet {
        print_banner(&config);
    }

    // Run the server
    crate::run_server(config).await
}

/// Execute the generate command
async fn execute_generate(cmd: GenerateCommand, config: SimulatorConfig) -> Result<()> {
    match cmd.action {
        GenerateAction::Chat {
            model,
            message,
            system,
            max_tokens,
            temperature,
            format,
            seed,
        } => {
            let mut engine_config = config;
            if let Some(s) = seed {
                engine_config.seed = Some(s);
            }

            let engine = SimulationEngine::new(engine_config);

            let mut messages = Vec::new();
            if let Some(sys) = system {
                messages.push(Message::system(&sys));
            }
            messages.push(Message::user(&message));

            let request = ChatCompletionRequest::new(model.clone(), messages)
                .with_options(
                    Some(temperature),
                    None,
                    Some(max_tokens),
                    false,
                    None,
                    None,
                    None,
                    None,
                );

            let response = engine.chat_completion(&request).await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                }
                "stream" => {
                    // Simulate streaming output
                    if let Some(choice) = response.choices.first() {
                        if let Some(content) = &choice.message.content {
                            for word in content.split_whitespace() {
                                print!("{} ", word);
                                io::stdout().flush()?;
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                            println!();
                        }
                    }
                }
                _ => {
                    // Text format
                    if let Some(choice) = response.choices.first() {
                        if let Some(content) = &choice.message.content {
                            println!("{}", content);
                        }
                    }
                }
            }

            Ok(())
        }

        GenerateAction::Embedding {
            model,
            text,
            dimensions,
            format,
        } => {
            let engine = SimulationEngine::new(config);

            let request = EmbeddingsRequest {
                model: model.clone(),
                input: EmbeddingInput::Single(text),
                encoding_format: None,
                dimensions: dimensions.map(|d| d as u32),
                user: None,
            };

            let response = engine.embeddings(&request).await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                }
                "base64" => {
                    if let Some(data) = response.data.first() {
                        let bytes: Vec<u8> = data.embedding.iter()
                            .flat_map(|f| f.to_le_bytes())
                            .collect();
                        let encoded = base64_encode(&bytes);
                        println!("{}", encoded);
                    }
                }
                _ => {
                    // Array format
                    if let Some(data) = response.data.first() {
                        println!("{:?}", data.embedding);
                    }
                }
            }

            Ok(())
        }

        GenerateAction::Config { format, full } => {
            let config = if full {
                SimulatorConfig::default()
            } else {
                // Minimal config
                SimulatorConfig::minimal()
            };

            let output = match format.as_str() {
                "toml" => toml::to_string_pretty(&config)?,
                "json" => serde_json::to_string_pretty(&config)?,
                _ => serde_yaml::to_string(&config)?,
            };

            println!("{}", output);
            Ok(())
        }

        GenerateAction::Requests {
            count,
            provider,
            output,
        } => {
            let requests = generate_sample_requests(count, &provider)?;
            let json = serde_json::to_string_pretty(&requests)?;

            if let Some(path) = output {
                std::fs::write(&path, &json)?;
                eprintln!("Generated {} requests to {:?}", count, path);
            } else {
                println!("{}", json);
            }

            Ok(())
        }
    }
}

/// Execute the config command
async fn execute_config(cmd: ConfigCommand, config: SimulatorConfig, _config_path: Option<PathBuf>) -> Result<()> {
    match cmd.action {
        ConfigAction::Show { format } => {
            let output = match format.as_str() {
                "toml" => toml::to_string_pretty(&config)?,
                "json" => serde_json::to_string_pretty(&config)?,
                _ => serde_yaml::to_string(&config)?,
            };
            println!("{}", output);
            Ok(())
        }

        ConfigAction::Validate { file } => {
            let config = SimulatorConfig::from_file(&file)?;
            config.validate()?;
            println!("Configuration at {:?} is valid", file);
            println!("  Models:    {}", config.models.len());
            println!("  Latency:   {}", if config.latency.enabled { "enabled" } else { "disabled" });
            println!("  Chaos:     {}", if config.chaos.enabled { "enabled" } else { "disabled" });
            Ok(())
        }

        ConfigAction::Init { output, preset, force } => {
            if output.exists() && !force {
                bail!("File {:?} already exists. Use --force to overwrite.", output);
            }

            let config = match preset.as_str() {
                "minimal" => SimulatorConfig::minimal(),
                "production" => SimulatorConfig::production(),
                _ => SimulatorConfig::default(),
            };

            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(&output, &yaml)?;
            println!("Created configuration file: {:?}", output);
            println!("Preset: {}", preset);
            Ok(())
        }

        ConfigAction::Models { provider } => {
            for (id, model_config) in &config.models {
                if let Some(ref p) = provider {
                    if model_config.provider.to_string() != *p {
                        continue;
                    }
                }
                println!("{:40} {:10} ctx:{:>6}", id, model_config.provider, model_config.context_length);
            }
            Ok(())
        }

        ConfigAction::Env => {
            println!("Environment Variable Mappings:");
            println!();
            println!("  {:<40} {}", "LLM_SIMULATOR_CONFIG", "Configuration file path");
            println!("  {:<40} {}", "LLM_SIMULATOR_PORT", "Server port (default: 8080)");
            println!("  {:<40} {}", "LLM_SIMULATOR_HOST", "Server host (default: 0.0.0.0)");
            println!("  {:<40} {}", "LLM_SIMULATOR_CHAOS", "Enable chaos engineering");
            println!("  {:<40} {}", "LLM_SIMULATOR_CHAOS_PROBABILITY", "Chaos probability (0.0-1.0)");
            println!("  {:<40} {}", "LLM_SIMULATOR_NO_LATENCY", "Disable latency simulation");
            println!("  {:<40} {}", "LLM_SIMULATOR_LATENCY_MULTIPLIER", "Latency multiplier");
            println!("  {:<40} {}", "LLM_SIMULATOR_SEED", "Random seed for determinism");
            println!("  {:<40} {}", "LLM_SIMULATOR_LOG_LEVEL", "Log level (trace/debug/info/warn/error)");
            println!("  {:<40} {}", "LLM_SIMULATOR_JSON_LOGS", "Enable JSON log format");
            println!("  {:<40} {}", "LLM_SIMULATOR_API_KEY", "API key for authentication");
            println!("  {:<40} {}", "LLM_SIMULATOR_REQUIRE_AUTH", "Require API key authentication");
            println!("  {:<40} {}", "LLM_SIMULATOR_MAX_CONCURRENT", "Max concurrent requests");
            println!("  {:<40} {}", "LLM_SIMULATOR_TIMEOUT", "Request timeout (seconds)");
            println!("  {:<40} {}", "OTEL_EXPORTER_OTLP_ENDPOINT", "OpenTelemetry OTLP endpoint");
            Ok(())
        }
    }
}

/// Execute the health command
async fn execute_health(cmd: HealthCommand) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cmd.timeout))
        .build()?;

    let endpoint = if cmd.ready { "ready" } else { "health" };

    loop {
        let url = format!("{}/{}", cmd.url.trim_end_matches('/'), endpoint);
        let start = Instant::now();

        match client.get(&url).send().await {
            Ok(response) => {
                let latency = start.elapsed();
                let status = response.status();
                let body: serde_json::Value = response.json().await.unwrap_or_default();

                match cmd.format.as_str() {
                    "json" => {
                        let result = serde_json::json!({
                            "url": url,
                            "status": status.as_u16(),
                            "latency_ms": latency.as_millis(),
                            "response": body,
                        });
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                    _ => {
                        let status_emoji = if status.is_success() { "✓" } else { "✗" };
                        println!("{} {} - Status: {} - Latency: {:?}",
                            status_emoji, url, status.as_u16(), latency);
                        if let Some(health_status) = body.get("status") {
                            println!("  Health: {}", health_status);
                        }
                    }
                }

                if !cmd.watch {
                    return if status.is_success() { Ok(()) } else {
                        bail!("Health check failed with status {}", status)
                    };
                }
            }
            Err(e) => {
                match cmd.format.as_str() {
                    "json" => {
                        let result = serde_json::json!({
                            "url": url,
                            "error": e.to_string(),
                        });
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                    _ => {
                        println!("✗ {} - Error: {}", url, e);
                    }
                }

                if !cmd.watch {
                    bail!("Health check failed: {}", e);
                }
            }
        }

        if cmd.watch {
            tokio::time::sleep(Duration::from_secs(cmd.interval)).await;
        }
    }
}

/// Execute the models command
async fn execute_models(cmd: ModelsCommand, config: SimulatorConfig) -> Result<()> {
    let models: Vec<ModelInfo> = if let Some(url) = cmd.url {
        // Fetch from remote instance
        let client = reqwest::Client::new();
        let response: ModelsResponse = client
            .get(format!("{}/v1/models", url.trim_end_matches('/')))
            .send()
            .await?
            .json()
            .await?;

        response.data.into_iter()
            .map(|m| ModelInfo {
                id: m.id.clone(),
                provider: m.owned_by.clone(),
                context_length: 0,
                capabilities: vec!["chat".to_string()],
            })
            .collect()
    } else {
        // Use local configuration
        config.models.iter()
            .map(|(id, mc)| {
                let mut capabilities = vec!["chat".to_string()];
                if mc.supports_streaming {
                    capabilities.push("streaming".to_string());
                }
                if mc.is_embedding {
                    capabilities = vec!["embedding".to_string()];
                }

                ModelInfo {
                    id: id.clone(),
                    provider: mc.provider.to_string(),
                    context_length: mc.context_length,
                    capabilities,
                }
            })
            .collect()
    };

    // Apply filters
    let filtered: Vec<_> = models.into_iter()
        .filter(|m| {
            if let Some(ref p) = cmd.provider {
                if !m.provider.to_lowercase().contains(&p.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref c) = cmd.capability {
                if !m.capabilities.iter().any(|cap| cap.contains(c)) {
                    return false;
                }
            }
            true
        })
        .collect();

    match cmd.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&filtered)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&filtered)?);
        }
        _ => {
            // Table format
            println!("{:<45} {:<12} {:>10} {}", "MODEL", "PROVIDER", "CONTEXT", "CAPABILITIES");
            println!("{}", "-".repeat(90));
            for m in filtered {
                println!("{:<45} {:<12} {:>10} {}",
                    m.id, m.provider, m.context_length, m.capabilities.join(", "));
            }
        }
    }

    Ok(())
}

/// Execute the benchmark command
async fn execute_benchmark(cmd: BenchmarkCommand) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let base_url = cmd.url.trim_end_matches('/');

    // Warmup
    eprintln!("Warming up with {} requests...", cmd.warmup);
    for _ in 0..cmd.warmup {
        let _ = send_benchmark_request(&client, base_url, &cmd.model, &cmd.request_type, cmd.stream).await;
    }

    eprintln!("Running benchmark: {} requests, {} concurrent", cmd.requests, cmd.concurrency);

    let semaphore = Arc::new(Semaphore::new(cmd.concurrency));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let total_latency_us = Arc::new(AtomicU64::new(0));
    let mut latencies = Vec::new();

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..cmd.requests {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let success_count = success_count.clone();
        let error_count = error_count.clone();
        let total_latency_us = total_latency_us.clone();
        let base_url = base_url.to_string();
        let model = cmd.model.clone();
        let request_type = cmd.request_type.clone();
        let stream = cmd.stream;

        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let req_start = Instant::now();

            match send_benchmark_request(&client, &base_url, &model, &request_type, stream).await {
                Ok(_) => {
                    let latency = req_start.elapsed();
                    success_count.fetch_add(1, Ordering::Relaxed);
                    total_latency_us.fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
                    Some(latency)
                }
                Err(_) => {
                    error_count.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        }));
    }

    for handle in handles {
        if let Some(latency) = handle.await? {
            latencies.push(latency);
        }
    }

    let elapsed = start.elapsed();
    let success = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    let rps = success as f64 / elapsed.as_secs_f64();

    latencies.sort();
    let p50 = latencies.get(latencies.len() / 2).copied().unwrap_or_default();
    let p95 = latencies.get(latencies.len() * 95 / 100).copied().unwrap_or_default();
    let p99 = latencies.get(latencies.len() * 99 / 100).copied().unwrap_or_default();

    let avg_latency = if !latencies.is_empty() {
        Duration::from_micros(total_latency_us.load(Ordering::Relaxed) / latencies.len() as u64)
    } else {
        Duration::ZERO
    };

    match cmd.format.as_str() {
        "json" => {
            let result = serde_json::json!({
                "requests": cmd.requests,
                "success": success,
                "errors": errors,
                "duration_secs": elapsed.as_secs_f64(),
                "rps": rps,
                "latency": {
                    "avg_ms": avg_latency.as_millis(),
                    "p50_ms": p50.as_millis(),
                    "p95_ms": p95.as_millis(),
                    "p99_ms": p99.as_millis(),
                }
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "csv" => {
            println!("requests,success,errors,duration_secs,rps,avg_ms,p50_ms,p95_ms,p99_ms");
            println!("{},{},{},{:.2},{:.2},{},{},{},{}",
                cmd.requests, success, errors, elapsed.as_secs_f64(), rps,
                avg_latency.as_millis(), p50.as_millis(), p95.as_millis(), p99.as_millis());
        }
        _ => {
            println!();
            println!("Benchmark Results");
            println!("=================");
            println!("Requests:     {}", cmd.requests);
            println!("Successful:   {}", success);
            println!("Errors:       {}", errors);
            println!("Duration:     {:.2}s", elapsed.as_secs_f64());
            println!("RPS:          {:.2}", rps);
            println!();
            println!("Latency:");
            println!("  Average:    {:?}", avg_latency);
            println!("  P50:        {:?}", p50);
            println!("  P95:        {:?}", p95);
            println!("  P99:        {:?}", p99);
        }
    }

    Ok(())
}

/// Execute the client command
async fn execute_client(cmd: ClientCommand) -> Result<()> {
    match cmd.action {
        ClientAction::Chat {
            url,
            model,
            message,
            system,
            max_tokens,
            temperature,
            stream,
            api_key,
            provider,
            raw,
        } => {
            let client = reqwest::Client::new();
            let base_url = url.trim_end_matches('/');

            // Read message from stdin if "-"
            let msg = if message == "-" {
                let mut input = String::new();
                io::stdin().lock().read_line(&mut input)?;
                input.trim().to_string()
            } else {
                message
            };

            let (endpoint, body) = build_chat_request(&provider, &model, &msg, system.as_deref(), max_tokens, temperature, stream);

            let mut req = client.post(format!("{}{}", base_url, endpoint))
                .json(&body);

            if let Some(key) = api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            if provider == "anthropic" {
                req = req.header("x-api-key", "test-key")
                    .header("anthropic-version", "2023-06-01");
            }

            let response = req.send().await?;
            let status = response.status();

            if raw || !status.is_success() {
                let body: serde_json::Value = response.json().await?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                let body: serde_json::Value = response.json().await?;
                let content = extract_content(&provider, &body);
                println!("{}", content);
            }

            Ok(())
        }

        ClientAction::Embed {
            url,
            model,
            text,
            api_key,
            raw,
        } => {
            let client = reqwest::Client::new();
            let base_url = url.trim_end_matches('/');

            let text = if text == "-" {
                let mut input = String::new();
                io::stdin().lock().read_line(&mut input)?;
                input.trim().to_string()
            } else {
                text
            };

            let body = serde_json::json!({
                "model": model,
                "input": text,
            });

            let mut req = client.post(format!("{}/v1/embeddings", base_url))
                .json(&body);

            if let Some(key) = api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            let response: serde_json::Value = req.send().await?.json().await?;

            if raw {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                if let Some(data) = response["data"].as_array() {
                    if let Some(first) = data.first() {
                        if let Some(embedding) = first["embedding"].as_array() {
                            println!("Dimensions: {}", embedding.len());
                            println!("First 5 values: {:?}", &embedding[..5.min(embedding.len())]);
                        }
                    }
                }
            }

            Ok(())
        }

        ClientAction::Interactive {
            url,
            model,
            system,
            api_key,
        } => {
            let client = reqwest::Client::new();
            let base_url = url.trim_end_matches('/');

            println!("Interactive Chat Session");
            println!("Model: {}", model);
            println!("Type 'exit' or 'quit' to end the session");
            println!();

            let mut messages: Vec<serde_json::Value> = Vec::new();

            if let Some(sys) = system {
                messages.push(serde_json::json!({
                    "role": "system",
                    "content": sys
                }));
            }

            loop {
                print!("You: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().lock().read_line(&mut input)?;
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                if input == "exit" || input == "quit" {
                    println!("Goodbye!");
                    break;
                }

                messages.push(serde_json::json!({
                    "role": "user",
                    "content": input
                }));

                let body = serde_json::json!({
                    "model": model,
                    "messages": messages,
                });

                let mut req = client.post(format!("{}/v1/chat/completions", base_url))
                    .json(&body);

                if let Some(ref key) = api_key {
                    req = req.header("Authorization", format!("Bearer {}", key));
                }

                match req.send().await {
                    Ok(response) => {
                        let body: serde_json::Value = response.json().await?;
                        let content = extract_content("openai", &body);
                        println!("Assistant: {}", content);
                        println!();

                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": content
                        }));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }

            Ok(())
        }
    }
}

/// Execute the version command
fn execute_version() -> Result<()> {
    println!("llm-simulator {}", VERSION);
    println!();
    println!("Build Information:");
    println!("  Version:       {}", VERSION);
    println!("  Rust Version:  {}", env!("CARGO_PKG_RUST_VERSION"));
    println!();
    println!("Features:");
    println!("  Multi-provider API simulation (OpenAI, Anthropic, Google)");
    println!("  Realistic latency simulation");
    println!("  Chaos engineering support");
    println!("  Deterministic execution with seeds");
    println!("  OpenTelemetry integration");
    println!("  Enterprise-grade security");
    Ok(())
}

// Helper functions

fn print_banner(config: &SimulatorConfig) {
    println!(r#"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ██╗     ██╗     ███╗   ███╗   ███████╗██╗███╗   ███╗       ║
║   ██║     ██║     ████╗ ████║   ██╔════╝██║████╗ ████║       ║
║   ██║     ██║     ██╔████╔██║   ███████╗██║██╔████╔██║       ║
║   ██║     ██║     ██║╚██╔╝██║   ╚════██║██║██║╚██╔╝██║       ║
║   ███████╗███████╗██║ ╚═╝ ██║   ███████║██║██║ ╚═╝ ██║       ║
║   ╚══════╝╚══════╝╚═╝     ╚═╝   ╚══════╝╚═╝╚═╝     ╚═╝       ║
║                                                               ║
║   LLM Simulator v{}                                       ║
║   Enterprise-grade offline LLM API simulator                  ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
"#, VERSION);

    println!("Configuration:");
    println!("  • Server:    {}:{}", config.server.host, config.server.port);
    println!("  • Models:    {} configured", config.models.len());
    println!("  • Latency:   {}", if config.latency.enabled { "enabled" } else { "disabled" });
    println!("  • Chaos:     {}", if config.chaos.enabled { "enabled" } else { "disabled" });
    println!("  • Seed:      {}", config.seed.map_or("random".to_string(), |s| s.to_string()));
    println!();
    println!("Endpoints:");
    println!("  • OpenAI:    http://{}:{}/v1/chat/completions", config.server.host, config.server.port);
    println!("  • Anthropic: http://{}:{}/v1/messages", config.server.host, config.server.port);
    println!("  • Google:    http://{}:{}/v1/models/{{model}}:generateContent", config.server.host, config.server.port);
    println!("  • Health:    http://{}:{}/health", config.server.host, config.server.port);
    println!("  • Metrics:   http://{}:{}/metrics", config.server.host, config.server.port);
    println!();
}

fn generate_sample_requests(count: usize, provider: &str) -> Result<Vec<serde_json::Value>> {
    let mut requests = Vec::with_capacity(count);
    let prompts = [
        "What is the capital of France?",
        "Explain quantum computing in simple terms.",
        "Write a haiku about programming.",
        "What are the benefits of Rust programming language?",
        "How does machine learning work?",
    ];

    for i in 0..count {
        let prompt = prompts[i % prompts.len()];
        let request = match provider {
            "anthropic" => serde_json::json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 256,
                "messages": [{"role": "user", "content": prompt}]
            }),
            "google" => serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": prompt}]}]
            }),
            _ => serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 256
            }),
        };
        requests.push(request);
    }

    Ok(requests)
}

async fn send_benchmark_request(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    request_type: &str,
    stream: bool,
) -> Result<()> {
    match request_type {
        "embedding" => {
            let body = serde_json::json!({
                "model": "text-embedding-ada-002",
                "input": "Hello, world!"
            });
            client.post(format!("{}/v1/embeddings", base_url))
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
        }
        _ => {
            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 10,
                "stream": stream
            });
            client.post(format!("{}/v1/chat/completions", base_url))
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
        }
    }
    Ok(())
}

fn build_chat_request(
    provider: &str,
    model: &str,
    message: &str,
    system: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
) -> (String, serde_json::Value) {
    match provider {
        "anthropic" => {
            let mut body = serde_json::json!({
                "model": model,
                "max_tokens": max_tokens.unwrap_or(256),
                "stream": stream,
                "messages": [{"role": "user", "content": message}]
            });
            if let Some(sys) = system {
                body["system"] = serde_json::json!(sys);
            }
            if let Some(temp) = temperature {
                body["temperature"] = serde_json::json!(temp);
            }
            ("/v1/messages".to_string(), body)
        }
        "google" => {
            let contents = vec![serde_json::json!({
                "role": "user",
                "parts": [{"text": message}]
            })];
            let body = serde_json::json!({
                "contents": contents,
                "generationConfig": {
                    "maxOutputTokens": max_tokens.unwrap_or(256),
                    "temperature": temperature.unwrap_or(1.0)
                }
            });
            (format!("/v1/models/{}:generateContent", model), body)
        }
        _ => {
            let mut messages = Vec::new();
            if let Some(sys) = system {
                messages.push(serde_json::json!({"role": "system", "content": sys}));
            }
            messages.push(serde_json::json!({"role": "user", "content": message}));

            let mut body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": stream
            });
            if let Some(mt) = max_tokens {
                body["max_tokens"] = serde_json::json!(mt);
            }
            if let Some(temp) = temperature {
                body["temperature"] = serde_json::json!(temp);
            }
            ("/v1/chat/completions".to_string(), body)
        }
    }
}

fn extract_content(provider: &str, body: &serde_json::Value) -> String {
    match provider {
        "anthropic" => {
            body["content"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["text"].as_str())
                .unwrap_or("")
                .to_string()
        }
        "google" => {
            body["candidates"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["content"]["parts"].as_array())
                .and_then(|parts| parts.first())
                .and_then(|p| p["text"].as_str())
                .unwrap_or("")
                .to_string()
        }
        _ => {
            body["choices"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["message"]["content"].as_str())
                .unwrap_or("")
                .to_string()
        }
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[derive(Debug, Clone, serde::Serialize)]
struct ModelInfo {
    id: String,
    provider: String,
    context_length: usize,
    capabilities: Vec<String>,
}
