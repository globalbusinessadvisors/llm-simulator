//! Streaming Support
//!
//! Real-time streaming response handling for LLM-Simulator SDK.

use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::types::*;
use super::{Client, SdkError, SdkResult};

/// Streaming chat builder
pub struct StreamingChat {
    client: Client,
    model: Option<String>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl StreamingChat {
    /// Create a new streaming chat builder
    pub fn new(client: Client) -> Self {
        let default_model = client.config().default_model.clone();
        let default_temp = client.config().default_temperature;
        let default_tokens = client.config().default_max_tokens;
        Self {
            client,
            model: default_model,
            messages: Vec::new(),
            temperature: default_temp,
            max_tokens: default_tokens,
        }
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content.into()));
        self
    }

    /// Add a user message
    pub fn message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content.into()));
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Start streaming and return a stream of chunks
    pub async fn start(self) -> SdkResult<ChatStream> {
        let model = self.model.ok_or_else(|| {
            SdkError::InvalidRequest("Model is required".to_string())
        })?;

        if self.messages.is_empty() {
            return Err(SdkError::InvalidRequest("At least one message is required".to_string()));
        }

        let request = ChatCompletionRequest::new(model.clone(), self.messages)
            .with_options(
                self.temperature,
                None, // top_p
                self.max_tokens,
                true, // stream
                None, // frequency_penalty
                None, // presence_penalty
                None, // stop
                None, // user
            );

        let endpoint = self.client.config().provider.chat_endpoint(&model);
        let url = format!("{}{}", self.client.config().base_url, endpoint);

        let mut req = self.client.http_client().post(&url)
            .json(&request);

        if let Some(ref key) = self.client.config().api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await.map_err(SdkError::from)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            return Err(SdkError::from_response(status, body));
        }

        Ok(ChatStream::new(response.bytes_stream(), model))
    }

    /// Stream and collect all content into a string
    pub async fn collect(self) -> SdkResult<String> {
        let mut stream = self.start().await?;
        let mut content = String::new();

        while let Some(chunk) = stream.next().await {
            if let Ok(c) = chunk {
                content.push_str(&c.content);
            }
        }

        Ok(content)
    }

    /// Stream with a callback for each chunk
    pub async fn for_each<F>(self, mut callback: F) -> SdkResult<StreamResult>
    where
        F: FnMut(&StreamChunk),
    {
        let mut stream = self.start().await?;
        let mut result = StreamResult::default();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    result.content.push_str(&chunk.content);
                    result.chunks += 1;
                    if chunk.finish_reason.is_some() {
                        result.finish_reason = chunk.finish_reason;
                    }
                    if let Some(usage) = &chunk.usage {
                        result.usage = Some(usage.clone());
                    }
                    callback(&chunk);
                }
                Err(e) => {
                    result.error = Some(e.to_string());
                    break;
                }
            }
        }

        Ok(result)
    }
}

/// A stream of chat completion chunks
pub struct ChatStream {
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    model: String,
    done: bool,
}

impl ChatStream {
    fn new(
        stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
        model: String,
    ) -> Self {
        Self {
            inner: Box::pin(stream),
            buffer: String::new(),
            model,
            done: false,
        }
    }

    /// Get the model being used
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Check if the stream is done
    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl Stream for ChatStream {
    type Item = SdkResult<StreamChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        loop {
            // Try to parse a complete SSE event from the buffer
            if let Some(chunk) = self.parse_buffer() {
                if chunk.content == "[DONE]" {
                    self.done = true;
                    return Poll::Ready(None);
                }
                return Poll::Ready(Some(Ok(chunk)));
            }

            // Need more data
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(SdkError::Stream(e.to_string()))));
                }
                Poll::Ready(None) => {
                    self.done = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl ChatStream {
    fn parse_buffer(&mut self) -> Option<StreamChunk> {
        // Look for complete SSE events
        while let Some(pos) = self.buffer.find("\n\n") {
            let event = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            // Parse SSE event
            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Some(StreamChunk {
                            content: "[DONE]".to_string(),
                            model: self.model.clone(),
                            finish_reason: None,
                            usage: None,
                        });
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        return Some(self.parse_chunk(&json));
                    }
                }
            }
        }

        None
    }

    fn parse_chunk(&self, json: &serde_json::Value) -> StreamChunk {
        let content = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["delta"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        let finish_reason = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["finish_reason"].as_str())
            .map(|s| match s {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Stop,
            });

        let usage = json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        StreamChunk {
            content,
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            finish_reason,
            usage,
        }
    }
}

/// A single chunk from a streaming response
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// The content of this chunk
    pub content: String,
    /// The model used
    pub model: String,
    /// Finish reason (if this is the final chunk)
    pub finish_reason: Option<FinishReason>,
    /// Usage information (usually in the final chunk)
    pub usage: Option<Usage>,
}

impl StreamChunk {
    /// Check if this is the final chunk
    pub fn is_final(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Check if the generation was stopped normally
    pub fn is_complete(&self) -> bool {
        matches!(self.finish_reason, Some(FinishReason::Stop))
    }

    /// Check if the generation was truncated due to length
    pub fn is_truncated(&self) -> bool {
        matches!(self.finish_reason, Some(FinishReason::Length))
    }
}

/// Result from streaming collection
#[derive(Debug, Clone, Default)]
pub struct StreamResult {
    /// Collected content
    pub content: String,
    /// Number of chunks received
    pub chunks: usize,
    /// Final finish reason
    pub finish_reason: Option<FinishReason>,
    /// Usage information
    pub usage: Option<Usage>,
    /// Any error that occurred
    pub error: Option<String>,
}

impl StreamResult {
    /// Check if streaming completed successfully
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the content
    pub fn text(&self) -> &str {
        &self.content
    }

    /// Get total tokens used
    pub fn total_tokens(&self) -> u32 {
        self.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk {
            content: "Hello".to_string(),
            model: "gpt-4".to_string(),
            finish_reason: None,
            usage: None,
        };

        assert!(!chunk.is_final());
        assert!(!chunk.is_truncated());

        let final_chunk = StreamChunk {
            content: "".to_string(),
            model: "gpt-4".to_string(),
            finish_reason: Some(FinishReason::Stop),
            usage: Some(Usage::new(10, 5)),
        };

        assert!(final_chunk.is_final());
        assert!(final_chunk.is_complete());
    }

    #[test]
    fn test_stream_result() {
        let result = StreamResult {
            content: "Hello, world!".to_string(),
            chunks: 3,
            finish_reason: Some(FinishReason::Stop),
            usage: Some(Usage::new(5, 3)),
            error: None,
        };

        assert!(result.is_success());
        assert_eq!(result.text(), "Hello, world!");
        assert_eq!(result.total_tokens(), 8);
    }
}
