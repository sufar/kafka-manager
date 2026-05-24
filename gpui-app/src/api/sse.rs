//! SSE Stream Module
//!
//! Implements SSE streaming for message queries.

use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;

/// SSE event types from the backend
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "start")]
    Start { partitions: usize, total_target: usize },
    #[serde(rename = "batch")]
    Batch { messages: Vec<StreamMessage>, progress: usize, total: usize },
    #[serde(rename = "order")]
    Order { sort: String },
    #[serde(rename = "complete")]
    Complete { actual_total: usize },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Stream message data
#[derive(Debug, Clone, Deserialize)]
pub struct StreamMessage {
    pub partition: i32,
    pub offset: i64,
    pub timestamp: i64,
    pub key: Option<String>,
    pub value: String,
    pub headers: Option<serde_json::Value>,
}

/// SSE stream configuration
#[derive(Clone)]
pub struct StreamConfig {
    pub cluster_id: i32,
    pub topic: String,
    pub partition: Option<i32>,
    pub mode: QueryMode,
    pub max_messages: i32,
    pub search_value: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

/// Query mode for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryMode {
    Newest,
    Oldest,
}

impl QueryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryMode::Newest => "newest",
            QueryMode::Oldest => "oldest",
        }
    }
}

/// SSE stream handle
#[allow(dead_code)]
pub struct MessageStream {
    #[allow(dead_code)]
    config: StreamConfig,
    #[allow(dead_code)]
    client: Arc<Client>,
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    abort_handle: Option<reqwest::RequestBuilder>,
}

impl MessageStream {
    /// Create a new message stream
    #[allow(dead_code)]
    pub fn new(config: StreamConfig, client: Arc<Client>, base_url: String) -> Self {
        Self {
            config,
            client,
            base_url,
            abort_handle: None,
        }
    }

    /// Start streaming messages
    /// Returns a channel that emits StreamEvent
    #[allow(dead_code)]
    pub async fn start(&self) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(100);

        let url = format!("{}/api/stream", self.base_url);

        let body = serde_json::json!({
            "cluster_id": self.config.cluster_id,
            "topic": self.config.topic,
            "partition": self.config.partition,
            "mode": self.config.mode.as_str(),
            "max_messages": self.config.max_messages,
            "search_value": self.config.search_value,
            "start_time": self.config.start_time,
            "end_time": self.config.end_time,
        });

        let client = self.client.clone();
        let tx_clone = tx.clone();

        // Spawn async task to handle SSE stream
        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("X-API-Method", "message.list")
                .json(&body)
                .send();

            match response.await {
                Ok(resp) => {
                    let mut stream = resp.bytes_stream();
                    let mut buffer = String::new();

                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                buffer.push_str(&String::from_utf8_lossy(&bytes));

                                // Parse SSE events from buffer
                                while let Some(event_end) = buffer.find("\n\n") {
                                    let event_str = buffer[..event_end].to_string();
                                    buffer = buffer[event_end + 2..].to_string();

                                    // Parse event
                                    if let Some(data) = parse_sse_event(&event_str) {
                                        if let Ok(event) = serde_json::from_str::<StreamEvent>(&data) {
                                            if tx_clone.send(event.clone()).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tx_clone.send(StreamEvent::Error {
                                    message: e.to_string()
                                }).await.ok();
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tx_clone.send(StreamEvent::Error {
                        message: e.to_string()
                    }).await.ok();
                }
            }
        });

        rx
    }
}

/// Parse SSE event to extract data
pub fn parse_sse_event(event_str: &str) -> Option<String> {
    for line in event_str.lines() {
        if line.starts_with("data: ") {
            return Some(line[6..].to_string());
        }
    }
    None
}

/// Message query state for UI
#[derive(Debug)]
pub struct MessageQueryState {
    pub is_querying: bool,
    pub total_received: usize,
    pub total_target: usize,
    pub messages: Vec<StreamMessage>,
    pub error: Option<String>,
    pub elapsed_ms: i64,
}

impl Default for MessageQueryState {
    fn default() -> Self {
        Self {
            is_querying: false,
            total_received: 0,
            total_target: 0,
            messages: Vec::new(),
            error: None,
            elapsed_ms: 0,
        }
    }
}