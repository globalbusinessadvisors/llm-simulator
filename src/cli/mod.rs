//! CLI Module for LLM-Simulator
//!
//! Provides a comprehensive command-line interface with subcommands for:
//! - Starting the simulator server
//! - Generating test data
//! - Managing configuration
//! - Health checking remote instances

mod commands;

pub use commands::*;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::VERSION;

/// LLM-Simulator: Enterprise-grade offline LLM API simulator
#[derive(Parser, Debug)]
#[command(name = "llm-simulator")]
#[command(author = "LLM DevOps Team")]
#[command(version = VERSION)]
#[command(about = "Enterprise-grade offline LLM API simulator for testing and development")]
#[command(propagate_version = true)]
pub struct Cli {
    /// Global configuration file path (YAML, TOML, or JSON)
    #[arg(short, long, global = true, env = "LLM_SIMULATOR_CONFIG")]
    pub config: Option<PathBuf>,

    /// Global log level (trace, debug, info, warn, error)
    #[arg(long, global = true, env = "LLM_SIMULATOR_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Enable JSON log output
    #[arg(long, global = true, env = "LLM_SIMULATOR_JSON_LOGS")]
    pub json_logs: bool,

    /// Quiet mode - suppress banner and non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the simulator server
    #[command(alias = "s")]
    Serve(ServeCommand),

    /// Generate test data or responses
    #[command(alias = "gen")]
    Generate(GenerateCommand),

    /// Configuration management
    #[command(alias = "cfg")]
    Config(ConfigCommand),

    /// Health check a running instance
    Health(HealthCommand),

    /// Show available models
    Models(ModelsCommand),

    /// Benchmark the simulator
    #[command(alias = "bench")]
    Benchmark(BenchmarkCommand),

    /// Run in client mode - send requests to a running instance
    Client(ClientCommand),

    /// Show version and build information
    Version,
}

/// Start the simulator server
#[derive(Parser, Debug)]
pub struct ServeCommand {
    /// Port to listen on
    #[arg(short, long, env = "LLM_SIMULATOR_PORT", default_value = "8080")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, env = "LLM_SIMULATOR_HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Enable chaos engineering
    #[arg(long, env = "LLM_SIMULATOR_CHAOS")]
    pub chaos: bool,

    /// Chaos probability (0.0-1.0)
    #[arg(long, env = "LLM_SIMULATOR_CHAOS_PROBABILITY")]
    pub chaos_probability: Option<f64>,

    /// Disable latency simulation
    #[arg(long, env = "LLM_SIMULATOR_NO_LATENCY")]
    pub no_latency: bool,

    /// Latency multiplier (1.0 = normal, 2.0 = 2x slower)
    #[arg(long, env = "LLM_SIMULATOR_LATENCY_MULTIPLIER")]
    pub latency_multiplier: Option<f64>,

    /// Set a fixed seed for deterministic behavior
    #[arg(long, env = "LLM_SIMULATOR_SEED")]
    pub seed: Option<u64>,

    /// OpenTelemetry endpoint for tracing
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Enable API key authentication
    #[arg(long, env = "LLM_SIMULATOR_REQUIRE_AUTH")]
    pub require_auth: bool,

    /// API key for authentication (if require_auth is enabled)
    #[arg(long, env = "LLM_SIMULATOR_API_KEY")]
    pub api_key: Option<String>,

    /// Maximum concurrent requests
    #[arg(long, env = "LLM_SIMULATOR_MAX_CONCURRENT")]
    pub max_concurrent: Option<usize>,

    /// Request timeout in seconds
    #[arg(long, env = "LLM_SIMULATOR_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Enable graceful shutdown with drain period (seconds)
    #[arg(long, env = "LLM_SIMULATOR_DRAIN_PERIOD")]
    pub drain_period: Option<u64>,

    /// Workers (defaults to number of CPUs)
    #[arg(long, env = "LLM_SIMULATOR_WORKERS")]
    pub workers: Option<usize>,
}

/// Generate test data or responses
#[derive(Parser, Debug)]
pub struct GenerateCommand {
    #[command(subcommand)]
    pub action: GenerateAction,
}

#[derive(Subcommand, Debug)]
pub enum GenerateAction {
    /// Generate a chat completion response
    Chat {
        /// Model to use
        #[arg(short, long, default_value = "gpt-4")]
        model: String,

        /// User message
        #[arg(short = 'u', long)]
        message: String,

        /// System prompt
        #[arg(short, long)]
        system: Option<String>,

        /// Maximum tokens to generate
        #[arg(long, default_value = "256")]
        max_tokens: u32,

        /// Temperature (0.0-2.0)
        #[arg(short, long, default_value = "1.0")]
        temperature: f32,

        /// Output format (json, text, stream)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Seed for deterministic output
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Generate embeddings
    Embedding {
        /// Model to use
        #[arg(short, long, default_value = "text-embedding-ada-002")]
        model: String,

        /// Text to embed
        #[arg(short, long)]
        text: String,

        /// Embedding dimensions
        #[arg(short, long)]
        dimensions: Option<usize>,

        /// Output format (json, base64, array)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Generate sample configuration
    Config {
        /// Output format (yaml, toml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Include all default values
        #[arg(long)]
        full: bool,
    },

    /// Generate sample requests for testing
    Requests {
        /// Number of requests to generate
        #[arg(short, long, default_value = "10")]
        count: usize,

        /// Provider format (openai, anthropic, google)
        #[arg(short, long, default_value = "openai")]
        provider: String,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Configuration management
#[derive(Parser, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show {
        /// Output format (yaml, toml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },

    /// Validate configuration file
    Validate {
        /// Configuration file to validate
        file: PathBuf,
    },

    /// Initialize a new configuration file
    Init {
        /// Output file path
        #[arg(short, long, default_value = "llm-simulator.yaml")]
        output: PathBuf,

        /// Configuration preset (minimal, standard, production)
        #[arg(short, long, default_value = "standard")]
        preset: String,

        /// Force overwrite existing file
        #[arg(short, long)]
        force: bool,
    },

    /// List available models
    Models {
        /// Filter by provider
        #[arg(short, long)]
        provider: Option<String>,
    },

    /// Show environment variable mappings
    Env,
}

/// Health check a running instance
#[derive(Parser, Debug)]
pub struct HealthCommand {
    /// Base URL of the simulator instance
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub url: String,

    /// Timeout in seconds
    #[arg(short, long, default_value = "5")]
    pub timeout: u64,

    /// Check readiness instead of liveness
    #[arg(short, long)]
    pub ready: bool,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Watch mode - continuously check health
    #[arg(short, long)]
    pub watch: bool,

    /// Watch interval in seconds
    #[arg(long, default_value = "5")]
    pub interval: u64,
}

/// Show available models
#[derive(Parser, Debug)]
pub struct ModelsCommand {
    /// Base URL of the simulator instance (if connecting to remote)
    #[arg(short, long)]
    pub url: Option<String>,

    /// Filter by provider (openai, anthropic, google)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Filter by capability (chat, embedding, streaming)
    #[arg(short, long)]
    pub capability: Option<String>,

    /// Output format (table, json, yaml)
    #[arg(short, long, default_value = "table")]
    pub format: String,
}

/// Benchmark the simulator
#[derive(Parser, Debug)]
pub struct BenchmarkCommand {
    /// Base URL of the simulator instance
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub url: String,

    /// Number of requests to send
    #[arg(short, long, default_value = "1000")]
    pub requests: usize,

    /// Concurrency level
    #[arg(short, long, default_value = "10")]
    pub concurrency: usize,

    /// Duration in seconds (alternative to request count)
    #[arg(short, long)]
    pub duration: Option<u64>,

    /// Model to benchmark
    #[arg(short, long, default_value = "gpt-4")]
    pub model: String,

    /// Request type (chat, embedding)
    #[arg(short = 't', long, default_value = "chat")]
    pub request_type: String,

    /// Enable streaming
    #[arg(short, long)]
    pub stream: bool,

    /// Output format (text, json, csv)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Warmup requests before benchmarking
    #[arg(long, default_value = "10")]
    pub warmup: usize,
}

/// Run in client mode - send requests to a running instance
#[derive(Parser, Debug)]
pub struct ClientCommand {
    #[command(subcommand)]
    pub action: ClientAction,
}

#[derive(Subcommand, Debug)]
pub enum ClientAction {
    /// Send a chat completion request
    Chat {
        /// Base URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,

        /// Model to use
        #[arg(short, long, default_value = "gpt-4")]
        model: String,

        /// User message (or - for stdin)
        message: String,

        /// System prompt
        #[arg(short, long)]
        system: Option<String>,

        /// Maximum tokens
        #[arg(long)]
        max_tokens: Option<u32>,

        /// Temperature
        #[arg(short, long)]
        temperature: Option<f32>,

        /// Enable streaming
        #[arg(short = 's', long)]
        stream: bool,

        /// API key
        #[arg(short, long, env = "LLM_SIMULATOR_API_KEY")]
        api_key: Option<String>,

        /// Provider API format (openai, anthropic, google)
        #[arg(short, long, default_value = "openai")]
        provider: String,

        /// Output raw JSON response
        #[arg(long)]
        raw: bool,
    },

    /// Send an embeddings request
    Embed {
        /// Base URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,

        /// Model to use
        #[arg(short, long, default_value = "text-embedding-ada-002")]
        model: String,

        /// Text to embed (or - for stdin)
        text: String,

        /// API key
        #[arg(short, long, env = "LLM_SIMULATOR_API_KEY")]
        api_key: Option<String>,

        /// Output raw JSON response
        #[arg(long)]
        raw: bool,
    },

    /// Interactive chat session
    Interactive {
        /// Base URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,

        /// Model to use
        #[arg(short, long, default_value = "gpt-4")]
        model: String,

        /// System prompt
        #[arg(short, long)]
        system: Option<String>,

        /// API key
        #[arg(short, long, env = "LLM_SIMULATOR_API_KEY")]
        api_key: Option<String>,
    },
}
