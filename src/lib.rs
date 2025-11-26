//! # LLM-Simulator
//!
//! Enterprise-grade offline LLM API simulator for testing and development.
//!
//! LLM-Simulator provides a drop-in replacement for production LLM APIs,
//! enabling cost-effective, deterministic, and comprehensive testing of
//! LLM-powered applications.
//!
//! ## Features
//!
//! - **Multi-Provider Support**: OpenAI, Anthropic, Google, Azure compatible APIs
//! - **Realistic Latency Simulation**: Statistical models for TTFT and ITL
//! - **Error Injection**: Chaos engineering for resilience testing
//! - **Deterministic Execution**: Reproducible tests with seed-based RNG
//! - **OpenTelemetry Integration**: Full observability support
//! - **High Performance**: 10,000+ RPS with <5ms overhead
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use llm_simulator::{SimulatorConfig, SimulationEngine, run_server};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = SimulatorConfig::default();
//!     run_server(config).await
//! }
//! ```

pub mod cli;
pub mod config;
pub mod engine;
pub mod error;
pub mod latency;
pub mod providers;
pub mod sdk;
pub mod security;
pub mod server;
pub mod telemetry;
pub mod types;

pub use config::SimulatorConfig;
pub use engine::SimulationEngine;
pub use error::{SimulationError, SimulatorResult};
pub use server::run_server;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default server port
pub const DEFAULT_PORT: u16 = 8080;

/// Default maximum concurrent requests
pub const DEFAULT_MAX_CONCURRENT: usize = 10_000;
