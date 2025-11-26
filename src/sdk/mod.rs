//! LLM-Simulator SDK
//!
//! A comprehensive client SDK for interacting with LLM-Simulator instances.
//!
//! # Features
//!
//! - **Async/Await Support**: Full async support with tokio
//! - **Multiple Providers**: OpenAI, Anthropic, and Google API formats
//! - **Streaming**: Real-time streaming response support
//! - **Connection Pooling**: Efficient HTTP connection management
//! - **Retry Logic**: Configurable retry with exponential backoff
//! - **Type Safety**: Strongly typed requests and responses
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use llm_simulator::sdk::{Client, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Client::new("http://localhost:8080")?;
//!
//!     let response = client
//!         .chat()
//!         .model("gpt-4")
//!         .message("Hello, world!")
//!         .send()
//!         .await?;
//!
//!     println!("{}", response.content());
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod builder;
mod error;
mod streaming;

pub use client::*;
pub use config::*;
pub use builder::*;
pub use error::*;
pub use streaming::*;

// Re-export types for convenience
pub use crate::types::{
    ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingsRequest, EmbeddingsResponse,
    Message, Role, Usage,
};
