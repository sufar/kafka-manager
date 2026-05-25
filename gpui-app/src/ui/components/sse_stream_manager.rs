//! SSE Stream Manager Component
//!
//! Manages real-time SSE message streaming from backend.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::api::{StreamConfig, StreamEvent, StreamMessage, QueryMode, SseStreamHandler};
use crate::state::{BufferedMessage, MessageBuffer};
use crate::ui::Theme;
use crate::i18n::Translations;

/// SSE Stream Manager - Handles real-time message streaming
pub struct SseStreamManager {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Stream handler
    handler: SseStreamHandler,
    /// Message buffer
    buffer: Arc<MessageBuffer<BufferedMessage>>,
    /// Current stream config
    config: Option<StreamConfig>,
    /// Stream state
    state: StreamState,
}

/// Stream state for UI display
#[derive(Debug, Clone, Default)]
pub struct StreamState {
    /// Is streaming active
    pub is_streaming: bool,
    /// Messages received count
    pub received: usize,
    /// Target messages count
    pub target: usize,
    /// Progress percentage (0.0 - 100.0)
    pub progress: f32,
    /// Elapsed time in ms
    pub elapsed_ms: i64,
    /// Error message if any
    pub error: Option<String>,
    /// Status text
    pub status_text: String,
}

impl StreamState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update from handler
    pub fn update_from_handler(&mut self, handler: &SseStreamHandler) {
        self.is_streaming = handler.is_streaming();
        self.received = handler.total_received();
        self.target = handler.total_target();
        self.progress = handler.progress();
        self.elapsed_ms = handler.elapsed_ms();
        self.error = handler.error().cloned();
        self.status_text = self.format_status();
    }

    /// Format status text
    fn format_status(&self) -> String {
        if self.is_streaming {
            format!("接收中: {} / {} ({:.0}%)", self.received, self.target, self.progress)
        } else if let Some(error) = &self.error {
            format!("错误: {}", error)
        } else if self.received > 0 {
            format!("已完成: {} 条消息", self.received)
        } else {
            "等待查询".to_string()
        }
    }

    /// Format elapsed time
    pub fn elapsed_text(&self) -> String {
        if self.elapsed_ms < 1000 {
            format!("{}ms", self.elapsed_ms)
        } else if self.elapsed_ms < 60000 {
            format!("{}s", self.elapsed_ms / 1000)
        } else {
            format!("{}m", self.elapsed_ms / 60000)
        }
    }
}

impl SseStreamManager {
    /// Create new SSE stream manager
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            handler: SseStreamHandler::new(),
            buffer: Arc::new(MessageBuffer::new()),
            config: None,
            state: StreamState::new(),
        }
    }

    /// Create with existing buffer
    pub fn with_buffer(theme: Theme, translations: Arc<Translations>, buffer: Arc<MessageBuffer<BufferedMessage>>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            handler: SseStreamHandler::new(),
            buffer,
            config: None,
            state: StreamState::new(),
        }
    }

    /// Start streaming with config
    pub fn start_stream(&mut self, config: StreamConfig) {
        let max_messages = config.max_messages as usize;
        self.config = Some(config.clone());
        self.handler.start_stream(config);
        self.state.is_streaming = true;
        self.state.target = max_messages;
        self.state.status_text = "启动流...".to_string();
    }

    /// Stop streaming
    pub fn stop_stream(&mut self) {
        self.handler.stop_stream();
        self.state.is_streaming = false;
        self.state.status_text = "已停止".to_string();
    }

    /// Handle stream event
    pub fn handle_event(&mut self, event: StreamEvent) {
        let messages = self.handler.handle_event(event.clone());

        // Add messages to buffer
        for msg in messages {
            // Note: In real implementation, buffer.push() would be called
            // self.buffer.push(msg);
        }

        // Update state
        self.state.update_from_handler(&self.handler);

        // Handle specific events
        match event {
            StreamEvent::Start { partitions, total_target } => {
                self.state.target = total_target;
                self.state.status_text = format!("开始接收 {} 个分区, 目标 {} 条消息", partitions, total_target);
            }
            StreamEvent::Batch { messages: msgs, progress, total } => {
                self.state.received = progress;
                self.state.target = total;
                // Add batch messages to buffer
                // In real impl: for msg in msgs { self.buffer.push(convert(msg)); }
            }
            StreamEvent::Complete { actual_total } => {
                self.state.is_streaming = false;
                self.state.received = actual_total;
                self.state.status_text = format!("完成: {} 条消息", actual_total);
            }
            StreamEvent::Error { message } => {
                self.state.is_streaming = false;
                self.state.error = Some(message.clone());
                self.state.status_text = format!("错误: {}", message);
            }
            StreamEvent::Order { sort } => {
                self.state.status_text = format!("排序: {}", sort);
            }
        }
    }

    /// Get current state
    pub fn state(&self) -> &StreamState {
        &self.state
    }

    /// Get buffer
    pub fn buffer(&self) -> &Arc<MessageBuffer<BufferedMessage>> {
        &self.buffer
    }

    /// Get handler
    pub fn handler(&self) -> &SseStreamHandler {
        &self.handler
    }

    /// Is streaming
    pub fn is_streaming(&self) -> bool {
        self.handler.is_streaming()
    }

    /// Get config
    pub fn config(&self) -> Option<&StreamConfig> {
        self.config.as_ref()
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        // self.buffer.clear();
        self.state = StreamState::new();
        self.config = None;
    }

    /// Render progress bar
    fn render_progress_bar(&self) -> Div {
        let theme = &self.theme;
        let progress_pct = self.state.progress / 100.0;
        let bar_width = if self.state.target > 0 {
            progress_pct * 200.0
        } else {
            0.0
        };

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                // Progress bar background
                div()
                    .flex_1()
                    .h(px(4.0))
                    .rounded(px(2.0))
                    .bg(theme.surface_raised)
                    .child(
                        // Progress fill
                        div()
                            .w(px(bar_width))
                            .h(px(4.0))
                            .rounded(px(2.0))
                            .bg(theme.primary)
                    )
            )
    }

    /// Render streaming status panel
    pub fn render_status_panel(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Status indicator
                div()
                    .w(px(10.0))
                    .h(px(10.0))
                    .rounded(px(5.0))
                    .bg(if self.state.is_streaming {
                        theme.success
                    } else if self.state.error.is_some() {
                        theme.error
                    } else {
                        theme.text_muted
                    })
            )
            .child(
                // Status text
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(self.state.status_text.clone())
            )
            .child(self.render_progress_bar())
            .when(self.state.is_streaming, |this| {
                this.child(
                    // Elapsed time
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(self.state.elapsed_text())
                )
            })
    }

    /// Render streaming controls
    pub fn render_controls(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                // Start/Stop button
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(if self.state.is_streaming { theme.error } else { theme.primary })
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                            .text_sm()
                            .child(if self.state.is_streaming {
                                t.messages.stop.clone()
                            } else {
                                t.messages.query.clone()
                            })
                    )
            )
            .child(
                // Clear button
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.surface_raised)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_sm()
                            .child("清除")
                    )
            )
    }
}

impl IntoElement for SseStreamManager {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .child(self.render_controls())
            .when(self.state.is_streaming || self.state.received > 0, |this| {
                this.child(self.render_status_panel())
            })
    }
}

impl Clone for SseStreamManager {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            handler: self.handler.clone(),
            buffer: self.buffer.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
        }
    }
}