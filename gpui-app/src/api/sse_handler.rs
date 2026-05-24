//! SSE Stream Handler
//!
//! Handles real-time message streaming from backend with memory management.

use gpui::*;

use crate::api::{StreamEvent, StreamMessage, StreamConfig};
use crate::state::BufferedMessage;

/// SSE stream state for UI
#[derive(Clone)]
pub struct SseStreamState {
    /// Whether stream is active
    is_streaming: bool,
    /// Total messages received
    total_received: usize,
    /// Target messages
    total_target: usize,
    /// Current progress percentage
    progress: f32,
    /// Elapsed time in ms
    elapsed_ms: i64,
    /// Error message if any
    error: Option<String>,
    /// Stream config
    config: Option<StreamConfig>,
}

impl Default for SseStreamState {
    fn default() -> Self {
        Self {
            is_streaming: false,
            total_received: 0,
            total_target: 0,
            progress: 0.0,
            elapsed_ms: 0,
            error: None,
            config: None,
        }
    }
}

impl SseStreamState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start streaming
    pub fn start(&mut self, config: StreamConfig) {
        self.is_streaming = true;
        self.total_received = 0;
        self.total_target = config.max_messages as usize;
        self.progress = 0.0;
        self.elapsed_ms = 0;
        self.error = None;
        self.config = Some(config);
    }

    /// Stop streaming
    pub fn stop(&mut self) {
        self.is_streaming = false;
        self.config = None;
    }

    /// Update progress
    pub fn update_progress(&mut self, received: usize, total: usize) {
        self.total_received = received;
        self.total_target = total;
        if total > 0 {
            self.progress = received as f32 / total as f32 * 100.0;
        }
    }

    /// Set error
    pub fn set_error(&mut self, error: String) {
        self.is_streaming = false;
        self.error = Some(error);
    }

    /// Convert stream message to buffered message
    pub fn convert_message(msg: StreamMessage) -> BufferedMessage {
        BufferedMessage {
            partition: msg.partition,
            offset: msg.offset,
            timestamp: msg.timestamp,
            key: msg.key,
            value: msg.value,
            headers: msg.headers.and_then(|h| {
                match h {
                    serde_json::Value::Object(obj) => {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.to_string()))
                            .collect::<Vec<_>>()
                            .into()
                    }
                    _ => None,
                }
            }).unwrap_or_default(),
        }
    }
}

/// SSE stream handler component
#[derive(Clone)]
pub struct SseStreamHandler {
    /// Stream state
    state: SseStreamState,
    /// Start time
    start_time: Option<i64>,
}

impl SseStreamHandler {
    /// Create new handler
    pub fn new() -> Self {
        Self {
            state: SseStreamState::new(),
            start_time: None,
        }
    }

    /// Check if streaming
    pub fn is_streaming(&self) -> bool {
        self.state.is_streaming
    }

    /// Get progress
    pub fn progress(&self) -> f32 {
        self.state.progress
    }

    /// Get total received
    pub fn total_received(&self) -> usize {
        self.state.total_received
    }

    /// Get total target
    pub fn total_target(&self) -> usize {
        self.state.total_target
    }

    /// Get elapsed time
    pub fn elapsed_ms(&self) -> i64 {
        self.state.elapsed_ms
    }

    /// Get error
    pub fn error(&self) -> Option<&String> {
        self.state.error.as_ref()
    }

    /// Start a new stream
    pub fn start_stream(&mut self, config: StreamConfig) {
        self.state.start(config);
        self.start_time = Some(current_timestamp_ms());
    }

    /// Stop current stream
    pub fn stop_stream(&mut self) {
        self.state.stop();
        self.start_time = None;
    }

    /// Handle stream event
    pub fn handle_event(&mut self, event: StreamEvent) -> Vec<BufferedMessage> {
        match event {
            StreamEvent::Start { partitions: _, total_target } => {
                self.state.total_target = total_target;
                Vec::new()
            }
            StreamEvent::Batch { messages, progress, total } => {
                self.state.update_progress(progress, total);
                if let Some(start) = self.start_time {
                    self.state.elapsed_ms = current_timestamp_ms() - start;
                }
                messages.into_iter()
                    .map(SseStreamState::convert_message)
                    .collect()
            }
            StreamEvent::Complete { actual_total } => {
                self.state.is_streaming = false;
                self.state.total_received = actual_total;
                if let Some(start) = self.start_time {
                    self.state.elapsed_ms = current_timestamp_ms() - start;
                }
                Vec::new()
            }
            StreamEvent::Error { message } => {
                self.state.set_error(message);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

impl Default for SseStreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for SseStreamHandler {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // This is a state-only component
        div().id("sse-handler-hidden")
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}