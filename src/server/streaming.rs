//! Streaming response implementation

use axum::response::sse::Event;
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::sleep;

use crate::engine::StreamingResponse;
use crate::types::*;

/// Sanitize JSON for SSE - remove newlines and carriage returns that would break SSE format
fn sanitize_sse_data(data: &str) -> String {
    // SSE data field can't contain raw newlines - they need to be escaped
    // The JSON serializer should handle this, but some content might have literal newlines
    data.replace('\n', "\\n").replace('\r', "\\r")
}

/// Create an SSE stream for OpenAI-compatible responses
pub fn create_sse_stream(
    response: StreamingResponse,
) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let chunks = response.into_chunks();

    Box::pin(stream::unfold(
        (chunks.into_iter(), false),
        |(mut iter, done)| async move {
            if done {
                return None;
            }

            match iter.next() {
                Some((delay, chunk)) => {
                    // Apply the delay
                    if delay > Duration::ZERO {
                        sleep(delay).await;
                    }

                    let data = serde_json::to_string(&chunk).unwrap_or_default();
                    let event = Event::default().data(data);
                    Some((Ok(event), (iter, false)))
                }
                None => {
                    // Send final [DONE] marker
                    let event = Event::default().data("[DONE]");
                    Some((Ok(event), (iter, true)))
                }
            }
        },
    ))
}

/// Create an SSE stream for Anthropic-compatible responses
pub fn create_anthropic_sse_stream(
    response: StreamingResponse,
    model: &str,
) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let id = response.id.clone();
    let model = model.to_string();
    let chunks = response.into_chunks();
    let usage = Usage::new(
        chunks.iter().map(|(_, c)| c.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0)).max().unwrap_or(0),
        chunks.iter().map(|(_, c)| c.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0)).max().unwrap_or(0),
    );

    Box::pin(stream::unfold(
        (chunks.into_iter(), 0usize, id, model, usage, false, false),
        |(mut iter, index, id, model, usage, started, done)| async move {
            if done {
                return None;
            }

            // Send message_start first
            if !started {
                let event = AnthropicStreamEvent::MessageStart {
                    message: AnthropicStreamMessage {
                        id: id.clone(),
                        message_type: "message".to_string(),
                        role: "assistant".to_string(),
                        content: vec![],
                        model: model.clone(),
                        stop_reason: None,
                        stop_sequence: None,
                        usage: AnthropicStreamUsage {
                            input_tokens: usage.prompt_tokens,
                            output_tokens: 0,
                        },
                    },
                };
                let data = sanitize_sse_data(&serde_json::to_string(&event).unwrap_or_default());
                let sse_event = Event::default()
                    .event("message_start")
                    .data(data);
                return Some((Ok(sse_event), (iter, 0, id, model, usage, true, false)));
            }

            match iter.next() {
                Some((delay, chunk)) => {
                    if delay > Duration::ZERO {
                        sleep(delay).await;
                    }

                    // Convert to Anthropic format
                    let content = chunk.choices.first()
                        .and_then(|c| c.delta.content.clone())
                        .unwrap_or_default();

                    if content.is_empty() && chunk.choices.first().and_then(|c| c.finish_reason).is_none() {
                        // Skip empty deltas that aren't finish markers
                        return Some((
                            Ok(Event::default().event("ping").data("{}")),
                            (iter, index, id, model, usage, started, false)
                        ));
                    }

                    if chunk.choices.first().and_then(|c| c.finish_reason).is_some() {
                        // Send content_block_stop, message_delta, and message_stop
                        let stop_event = AnthropicStreamEvent::ContentBlockStop { index: 0 };
                        let data = sanitize_sse_data(&serde_json::to_string(&stop_event).unwrap_or_default());
                        let sse_event = Event::default()
                            .event("content_block_stop")
                            .data(data);
                        return Some((Ok(sse_event), (iter, index, id, model, usage, started, true)));
                    }

                    // Send content_block_start if this is first content
                    if index == 0 {
                        let start_event = AnthropicStreamEvent::ContentBlockStart {
                            index: 0,
                            content_block: AnthropicContentBlockType::Text { text: String::new() },
                        };
                        let data = sanitize_sse_data(&serde_json::to_string(&start_event).unwrap_or_default());
                        let sse_event = Event::default()
                            .event("content_block_start")
                            .data(data);
                        // We need to send content delta next
                        return Some((Ok(sse_event), (iter, index + 1, id, model, usage, started, false)));
                    }

                    // Send content delta
                    let delta_event = AnthropicStreamEvent::ContentBlockDelta {
                        index: 0,
                        delta: AnthropicDelta::TextDelta { text: content },
                    };
                    let data = sanitize_sse_data(&serde_json::to_string(&delta_event).unwrap_or_default());
                    let sse_event = Event::default()
                        .event("content_block_delta")
                        .data(data);
                    Some((Ok(sse_event), (iter, index + 1, id, model, usage, started, false)))
                }
                None => {
                    // Send message_stop
                    let event = AnthropicStreamEvent::MessageStop;
                    let data = sanitize_sse_data(&serde_json::to_string(&event).unwrap_or_default());
                    let sse_event = Event::default()
                        .event("message_stop")
                        .data(data);
                    Some((Ok(sse_event), (iter, index, id, model, usage, started, true)))
                }
            }
        },
    ))
}

/// Create an SSE stream for Gemini-compatible responses
pub fn create_gemini_sse_stream(
    response: StreamingResponse,
) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let chunks = response.into_chunks();

    Box::pin(stream::unfold(
        (chunks.into_iter(), String::new()),
        |(mut iter, mut accumulated)| async move {
            match iter.next() {
                Some((delay, chunk)) => {
                    if delay > Duration::ZERO {
                        sleep(delay).await;
                    }

                    let content = chunk.choices.first()
                        .and_then(|c| c.delta.content.clone())
                        .unwrap_or_default();

                    accumulated.push_str(&content);

                    // Create Gemini-style response
                    let gemini_chunk = GeminiResponse {
                        candidates: vec![GeminiCandidate {
                            content: GeminiResponseContent {
                                role: "model".to_string(),
                                parts: vec![GeminiResponsePart { text: content }],
                            },
                            finish_reason: chunk.choices.first()
                                .and_then(|c| c.finish_reason)
                                .map(|_| "STOP".to_string()),
                            safety_ratings: None,
                        }],
                        usage_metadata: chunk.usage.map(|u| GeminiUsageMetadata {
                            prompt_token_count: u.prompt_tokens,
                            candidates_token_count: u.completion_tokens,
                            total_token_count: u.total_tokens,
                        }),
                    };

                    let data = serde_json::to_string(&gemini_chunk).unwrap_or_default();
                    let event = Event::default().data(data);
                    Some((Ok(event), (iter, accumulated)))
                }
                None => None,
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latency::LatencySchedule;
    use futures::StreamExt;

    fn test_streaming_response() -> StreamingResponse {
        StreamingResponse {
            id: "test-id".to_string(),
            model: "gpt-4".to_string(),
            tokens: vec!["Hello".to_string(), " ".to_string(), "World".to_string()],
            schedule: LatencySchedule::instant(3),
            usage: Usage::new(10, 3),
        }
    }

    #[tokio::test]
    async fn test_openai_stream() {
        let response = test_streaming_response();
        let mut stream = create_sse_stream(response);

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }

        // Should have role chunk, content chunks, finish chunk, and [DONE]
        assert!(events.len() >= 4);
    }

    #[tokio::test]
    async fn test_anthropic_stream() {
        let response = test_streaming_response();
        let mut stream = create_anthropic_sse_stream(response, "claude-3");

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }

        // Should have message_start and more events
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_gemini_stream() {
        let response = test_streaming_response();
        let mut stream = create_gemini_sse_stream(response);

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }

        assert!(!events.is_empty());
    }
}
