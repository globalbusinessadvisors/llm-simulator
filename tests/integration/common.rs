//! Common test utilities for integration tests
//!
//! Provides test server spawning, request builders, and assertions.

use std::net::SocketAddr;
use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use llm_simulator::{
    config::SimulatorConfig,
    security::SecurityState,
    server::{create_router, AppState},
};

/// Test server wrapper
pub struct TestServer {
    pub addr: SocketAddr,
    pub client: Client,
    pub base_url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TestServer {
    /// Spawn a test server with default configuration
    pub async fn spawn() -> Self {
        Self::spawn_with_config(SimulatorConfig::default()).await
    }

    /// Spawn a test server with custom configuration
    pub async fn spawn_with_config(mut config: SimulatorConfig) -> Self {
        // Find an available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        config.server.host = "127.0.0.1".to_string();
        config.server.port = addr.port();

        // Create state and router
        let state = AppState::new(config.clone());
        let security = SecurityState::new(&config.security);
        let app = create_router(state, security);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn server
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        // Wait for server to be ready
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        let base_url = format!("http://{}", addr);

        // Wait for health endpoint
        for _ in 0..50 {
            if client.get(format!("{}/health", base_url)).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Self {
            addr,
            client,
            base_url,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Get the base URL
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Send a GET request
    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(self.url(path)).send().await.unwrap()
    }

    /// Send a POST request with JSON body
    pub async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    /// Send a POST request with authorization
    pub async fn post_with_auth(&self, path: &str, body: Value, api_key: &str) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Create a simple chat request
pub fn chat_request(model: &str, message: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": message
            }
        ]
    })
}

/// Create a chat request with options
pub fn chat_request_with_options(
    model: &str,
    messages: Vec<Value>,
    max_tokens: Option<i32>,
    temperature: Option<f32>,
    stream: bool,
) -> Value {
    let mut req = json!({
        "model": model,
        "messages": messages,
        "stream": stream
    });

    if let Some(mt) = max_tokens {
        req["max_tokens"] = json!(mt);
    }
    if let Some(temp) = temperature {
        req["temperature"] = json!(temp);
    }

    req
}

/// Create an Anthropic messages request
pub fn anthropic_request(model: &str, message: &str, max_tokens: i32) -> Value {
    json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [
            {
                "role": "user",
                "content": message
            }
        ]
    })
}

/// Create a Google Gemini request
pub fn gemini_request(message: &str) -> Value {
    json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "text": message
                    }
                ]
            }
        ]
    })
}

/// Create an embeddings request
pub fn embeddings_request(model: &str, input: &str) -> Value {
    json!({
        "model": model,
        "input": input
    })
}

/// Assert response status
pub fn assert_status(response: &reqwest::Response, expected: u16) {
    assert_eq!(
        response.status().as_u16(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}

/// Assert JSON field exists
pub fn assert_json_field(json: &Value, field: &str) {
    assert!(
        json.get(field).is_some(),
        "Expected field '{}' to exist in {:?}",
        field,
        json
    );
}

/// Assert JSON field equals value
pub fn assert_json_eq(json: &Value, field: &str, expected: &Value) {
    assert_eq!(
        json.get(field),
        Some(expected),
        "Expected {}={:?}, got {:?}",
        field,
        expected,
        json.get(field)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_server() {
        let server = TestServer::spawn().await;
        let response = server.get("/health").await;
        assert_eq!(response.status().as_u16(), 200);
    }

    #[test]
    fn test_chat_request_builder() {
        let req = chat_request("gpt-4", "Hello");
        assert_eq!(req["model"], "gpt-4");
        assert_eq!(req["messages"][0]["content"], "Hello");
    }
}
