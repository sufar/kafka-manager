//! Memory Efficient Message Buffer
//!
//! Optimized circular buffer for storing Kafka messages with memory limits.

use std::collections::VecDeque;

/// Configuration for the message buffer
#[derive(Debug, Clone)]
pub struct MessageBufferConfig {
    /// Maximum number of messages to store
    pub max_messages: usize,
    /// Maximum memory usage in bytes (approximate)
    pub max_memory_bytes: usize,
    /// Whether to auto-evict old messages when limit reached
    pub auto_evict: bool,
}

impl Default for MessageBufferConfig {
    fn default() -> Self {
        Self {
            max_messages: 10_000,
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            auto_evict: true,
        }
    }
}

/// A memory-efficient circular buffer for messages
pub struct MessageBuffer<T> {
    /// The circular buffer storage
    buffer: VecDeque<T>,
    /// Configuration
    config: MessageBufferConfig,
    /// Estimated memory usage
    estimated_memory: usize,
    /// Total messages received (including evicted)
    total_received: usize,
    /// Messages evicted due to memory limits
    evicted_count: usize,
}

impl<T: MessageSizeEstimate> MessageBuffer<T> {
    /// Create a new message buffer with default config
    pub fn new() -> Self {
        Self::with_config(MessageBufferConfig::default())
    }

    /// Create a new message buffer with custom config
    pub fn with_config(config: MessageBufferConfig) -> Self {
        Self {
            buffer: VecDeque::with_capacity(config.max_messages / 2),
            config,
            estimated_memory: 0,
            total_received: 0,
            evicted_count: 0,
        }
    }

    /// Add a message to the buffer
    pub fn push(&mut self, message: T) {
        let size = message.estimated_size();

        // Check if we need to evict old messages
        if self.config.auto_evict {
            while self.buffer.len() >= self.config.max_messages
                || self.estimated_memory + size > self.config.max_memory_bytes
                && self.buffer.len() > 0 {
                if let Some(evicted) = self.buffer.pop_front() {
                    self.estimated_memory -= evicted.estimated_size();
                    self.evicted_count += 1;
                }
            }
        }

        // Add the new message
        self.buffer.push_back(message);
        self.estimated_memory += size;
        self.total_received += 1;
    }

    /// Get all messages in the buffer
    pub fn messages(&self) -> &VecDeque<T> {
        &self.buffer
    }

    /// Get a message by index (relative to buffer start)
    pub fn get(&self, index: usize) -> Option<&T> {
        self.buffer.get(index)
    }

    /// Get the number of messages in the buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get total messages received
    pub fn total_received(&self) -> usize {
        self.total_received
    }

    /// Get messages evicted count
    pub fn evicted_count(&self) -> usize {
        self.evicted_count
    }

    /// Get estimated memory usage
    pub fn estimated_memory(&self) -> usize {
        self.estimated_memory
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.estimated_memory = 0;
    }

    /// Get visible range for virtual scrolling
    pub fn visible_range(&self, scroll_offset: f32, item_height: f32, container_height: f32) -> (usize, usize) {
        if self.buffer.is_empty() {
            return (0, 0);
        }

        let first_visible = (scroll_offset / item_height) as usize;
        let visible_count = (container_height / item_height) as usize + 5; // buffer 5 items

        let start = first_visible.max(0);
        let end = (start + visible_count).min(self.buffer.len());

        (start, end)
    }

    /// Get messages in visible range
    pub fn visible_messages(&self, start: usize, end: usize) -> Vec<&T> {
        self.buffer.iter()
            .skip(start)
            .take(end - start)
            .collect()
    }
}

impl<T: MessageSizeEstimate> Default for MessageBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for estimating message size
pub trait MessageSizeEstimate {
    /// Estimate the memory size of this message in bytes
    fn estimated_size(&self) -> usize;
}

/// Implementation for a simple message type
#[derive(Debug, Clone)]
pub struct BufferedMessage {
    pub partition: i32,
    pub offset: i64,
    pub timestamp: i64,
    pub key: Option<String>,
    pub value: String,
    pub headers: Vec<(String, String)>,
}

impl MessageSizeEstimate for BufferedMessage {
    fn estimated_size(&self) -> usize {
        // Base struct size (approximate)
        let base: usize = 32; // partition + offset + timestamp + pointers

        // Key size
        let key_size: usize = self.key.as_ref().map(|k| k.len()).unwrap_or(0);

        // Value size
        let value_size: usize = self.value.len();

        // Headers size
        let headers_size: usize = self.headers.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();

        base + key_size + value_size + headers_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_push_and_evict() {
        let config = MessageBufferConfig {
            max_messages: 5,
            max_memory_bytes: 1000,
            auto_evict: true,
        };

        let mut buffer: MessageBuffer<BufferedMessage> = MessageBuffer::with_config(config);

        // Add messages
        for i in 0..10 {
            buffer.push(BufferedMessage {
                partition: 0,
                offset: i,
                timestamp: 0,
                key: None,
                value: "test".to_string(),
                headers: vec![],
            });
        }

        // Should have evicted some messages
        assert!(buffer.evicted_count() > 0);
        assert!(buffer.len() <= 5);
        assert!(buffer.total_received() == 10);
    }

    #[test]
    fn test_visible_range() {
        let mut buffer: MessageBuffer<BufferedMessage> = MessageBuffer::new();

        // Add 100 messages
        for i in 0..100 {
            buffer.push(BufferedMessage {
                partition: 0,
                offset: i,
                timestamp: 0,
                key: None,
                value: "test".to_string(),
                headers: vec![],
            });
        }

        // Test visible range
        let (start, end) = buffer.visible_range(0.0, 40.0, 400.0);
        assert!(start == 0);
        assert!(end > start);

        // Test scroll offset
        let (start, end) = buffer.visible_range(800.0, 40.0, 400.0); // scroll to 20th item
        assert!(start > 0);
        assert!(end > start);
    }
}