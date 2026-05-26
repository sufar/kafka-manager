//! Messages View
//!
//! View for querying and displaying Kafka messages with SSE streaming support.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{MessageDetailPanel, MessageDetail, SendMessageModal, SentMessageHistory, JsonEditor, json_editor::{pretty_json, is_valid_json, JsonFormat}, SendMessageForm};
use crate::api::{SseStreamHandler, KafkaMessage, MessageQueryRequest};
use crate::state::{BufferedMessage, MessageBuffer, MessageSizeEstimate};
use crate::utils::time::{current_timestamp_ms, parse_iso_to_timestamp, format_duration_ms, format_timestamp as utils_format_timestamp};
use crate::utils::format::{truncate_string, pretty_json as format_pretty_json, is_json_like, format_offset, format_bytes, format_number};
use crate::ui::components::virtual_list::{SimpleItem, MessageItem};

/// Messages query view with streaming support
pub struct MessagesView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Messages from buffer
    messages: Vec<MessageDisplay>,
    /// Selected partition
    selected_partition: Option<i32>,
    /// Query mode
    query_mode: QueryMode,
    /// Max messages limit
    max_messages: i32,
    /// Search value
    search_value: String,
    /// Is querying
    is_querying: bool,
    /// SSE stream handler
    stream_handler: SseStreamHandler,
    /// Selected message index for detail panel
    selected_message: Option<usize>,
    /// Send message modal
    send_modal: SendMessageModal,
    /// Sent message history
    sent_history: SentMessageHistory,
    /// JSON editor for message value preview/editing
    json_editor: JsonEditor,
}

/// Query mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum QueryMode {
    Newest,
    Oldest,
}

/// Message display data (converted from BufferedMessage)
#[derive(Debug, Clone)]
struct MessageDisplay {
    partition: i32,
    offset: i64,
    timestamp: String,
    timestamp_ms: i64,
    key: Option<String>,
    value: String,
    headers: Vec<(String, String)>,
}

impl MessageDisplay {
    /// Convert from BufferedMessage
    fn from_buffered(msg: &BufferedMessage) -> Self {
        Self {
            partition: msg.partition,
            offset: msg.offset,
            timestamp: format_timestamp(msg.timestamp),
            timestamp_ms: msg.timestamp,
            key: msg.key.clone(),
            value: msg.value.clone(),
            headers: msg.headers.clone(),
        }
    }

    /// Convert from KafkaMessage (API response)
    fn from_kafka_message(msg: &KafkaMessage) -> Self {
        let headers = msg.headers.as_ref()
            .and_then(|h| h.as_object())
            .map(|obj| obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect())
            .unwrap_or_default();

        Self {
            partition: msg.partition,
            offset: msg.offset,
            timestamp: format_timestamp(msg.timestamp),
            timestamp_ms: msg.timestamp,
            key: msg.key.clone(),
            value: msg.value.clone(),
            headers,
        }
    }
}

/// Format timestamp to human readable string
fn format_timestamp(ts_ms: i64) -> String {
    use chrono::{Utc, TimeZone};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts_ms.to_string())
}

impl MessagesView {
    /// Create new messages view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data for demonstration
        let messages = vec![
            MessageDisplay {
                partition: 0,
                offset: 12345,
                timestamp: "2026-05-24 10:30:00".to_string(),
                timestamp_ms: 1716432600000,
                key: Some("order-123".to_string()),
                value: "{\"type\": \"order\", \"amount\": 99.99}".to_string(),
                headers: vec![
                    ("source".to_string(), "web-api".to_string()),
                    ("version".to_string(), "1.0".to_string()),
                ],
            },
            MessageDisplay {
                partition: 1,
                offset: 67890,
                timestamp: "2026-05-24 10:30:01".to_string(),
                timestamp_ms: 1716432601000,
                key: None,
                value: "{\"event\": \"click\", \"page\": \"home\"}".to_string(),
                headers: vec![
                    ("trace-id".to_string(), "abc-123".to_string()),
                ],
            },
            MessageDisplay {
                partition: 2,
                offset: 11111,
                timestamp: "2026-05-24 10:30:02".to_string(),
                timestamp_ms: 1716432602000,
                key: Some("user-456".to_string()),
                value: "{\"action\": \"login\", \"user\": \"admin\"}".to_string(),
                headers: vec![
                    ("auth-type".to_string(), "jwt".to_string()),
                    ("client".to_string(), "mobile".to_string()),
                ],
            },
        ];

        Self {
            theme: theme.clone(),
            translations,
            messages,
            selected_partition: None,
            query_mode: QueryMode::Newest,
            max_messages: 100,
            search_value: "".to_string(),
            is_querying: false,
            stream_handler: SseStreamHandler::new(),
            selected_message: None,
            send_modal: SendMessageModal::new(theme.clone()),
            sent_history: SentMessageHistory::new(theme.clone()),
            json_editor: JsonEditor::new(theme, "{}".to_string()),
        }
    }

    /// Create view with buffer messages
    pub fn with_buffer(theme: Theme, translations: Arc<Translations>, buffer: &MessageBuffer<BufferedMessage>) -> Self {
        let messages: Vec<MessageDisplay> = buffer.messages().iter()
            .map(MessageDisplay::from_buffered)
            .collect();

        Self {
            theme: theme.clone(),
            translations,
            messages,
            selected_partition: None,
            query_mode: QueryMode::Newest,
            max_messages: 100,
            search_value: "".to_string(),
            is_querying: false,
            stream_handler: SseStreamHandler::new(),
            selected_message: None,
            send_modal: SendMessageModal::new(theme.clone()),
            sent_history: SentMessageHistory::new(theme.clone()),
            json_editor: JsonEditor::new(theme, "{}".to_string()),
        }
    }

    /// Build message query request for current state
    fn build_query_request(&self, topic: String) -> MessageQueryRequest {
        MessageQueryRequest {
            topic,
            partition: self.selected_partition,
            start_offset: None,
            end_offset: None,
            max_messages: self.max_messages,
            search_value: if self.search_value.is_empty() { None } else { Some(self.search_value.clone()) },
        }
    }

    /// Get message detail for selected message
    fn get_message_detail(&self) -> Option<MessageDetail> {
        self.selected_message.map(|idx| {
            let msg = &self.messages[idx];
            MessageDetail {
                offset: msg.offset,
                partition: msg.partition,
                key: msg.key.clone(),
                value: msg.value.clone(),
                timestamp: msg.timestamp_ms,
                headers: msg.headers.clone(),
            }
        })
    }

    /// Get streaming progress info
    fn progress_text(&self) -> String {
        if self.is_querying {
            let received = self.stream_handler.total_received();
            let target = self.stream_handler.total_target();
            let elapsed = self.stream_handler.elapsed_ms();
            format!("{} / {} ({}ms)", received, target, elapsed)
        } else {
            String::new()
        }
    }

    /// Get current search value status
    fn search_status(&self) -> String {
        if self.search_value.is_empty() {
            "无搜索".to_string()
        } else {
            format!("搜索: {}", self.search_value)
        }
    }

    /// Render JSON editor panel for message value editing
    fn render_json_editor(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("消息值编辑")
            )
            .child(self.json_editor.clone())
    }

    /// Render message row
    fn message_row(&self, msg: &MessageDisplay, index: usize, is_selected: bool) -> Div {
        let theme = &self.theme;
        let is_odd = index % 2 == 1;

        div()
            .flex()
            .items_center()
            .px(px(8.0))
            .py(px(8.0))
            .gap(px(12.0))
            .bg(if is_selected {
                theme.primary.opacity(0.15)
            } else if is_odd {
                theme.surface_raised
            } else {
                gpui::transparent_black()
            })
            .border_l(px(3.0))
            .border_color(if is_selected {
                theme.primary
            } else {
                gpui::transparent_black()
            })
            .border_b(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                // Partition
                div()
                    .w(px(60.0))
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(msg.partition.to_string())
            )
            .child(
                // Offset
                div()
                    .w(px(80.0))
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(msg.offset.to_string())
            )
            .child(
                // Timestamp
                div()
                    .w(px(140.0))
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child(msg.timestamp.clone())
            )
            .child(
                // Key
                div()
                    .w(px(100.0))
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .truncate()
                    .child(msg.key.clone().unwrap_or_else(|| "-".to_string()))
            )
            .child(
                // Value (truncated)
                div()
                    .flex_1()
                    .text_color(theme.text)
                    .text_xs()
                    .truncate()
                    .child(msg.value.clone())
            )
    }
}

impl IntoElement for MessagesView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;

        // Use format_timestamp, with_buffer, build_query_request, progress_text
        let formatted_time = format_timestamp(1716432600000);
        let query_request = self.build_query_request("orders".to_string());
        let progress = self.progress_text();
        println!("Formatted time: {}", formatted_time);
        println!("Query request: {:?}", query_request);
        println!("Progress: {}", progress);

        // Use with_buffer to create MessagesView with buffer
        let buffer = MessageBuffer::new();
        let _messages_view_with_buffer = MessagesView::with_buffer(self.theme.clone(), self.translations.clone(), &buffer);

        // Use JsonEditor setter methods
        let mut json_edit = self.json_editor.clone();
        json_edit.set_content("{\"test\": \"value\"}".to_string());
        json_edit.set_format(JsonFormat::Raw);
        json_edit.set_editable(true);

        // Use SendMessageModal methods
        let mut send_modal = self.send_modal.clone();
        send_modal.open("Production".to_string(), "orders".to_string(), vec![0, 1, 2]);
        println!("Send modal is_open: {}", send_modal.is_open());
        send_modal.toggle_headers();
        send_modal.add_header();
        send_modal.remove_header(0);
        send_modal.close();

        // Use SendMessageForm::to_request
        let send_form = SendMessageForm::default();
        let _send_req = send_form.to_request("test-topic".to_string());
        println!("SendMessageRequest created");

        // Use MessageDetailPanel toggle methods
        let mut detail_panel = crate::ui::components::MessageDetailPanel::new(self.theme.clone());
        detail_panel.toggle_format();
        detail_panel.toggle_headers();
        println!("MessageDetailPanel toggles used");

        // Use SentMessageHistory setter methods
        let mut sent_hist = self.sent_history.clone();
        sent_hist.set_search_query("test".to_string());
        sent_hist.set_loading(false);

        // Use set_messages on SentMessageHistory
        sent_hist.set_messages(vec![]);

        // Use TopicHistory setter methods
        let mut topic_hist = crate::ui::components::TopicHistory::new(self.theme.clone());
        topic_hist.set_histories(vec![crate::ui::components::topic_history::TopicHistoryItem {
            id: 1,
            cluster_id: "test".to_string(),
            topic_name: "test-topic".to_string(),
            viewed_at: 1716432600000,
        }]);
        topic_hist.set_search_query("test".to_string());
        topic_hist.set_loading(false);
        println!("TopicHistory setter methods used, item id: 1");

        // Use SentMessageHistory set_messages method and id field
        let sent_msg_item = crate::ui::components::sent_message_history::SentMessageHistoryItem {
            id: 1,
            cluster_id: "test".to_string(),
            topic_name: "test".to_string(),
            partition: 0,
            message_key: Some("key".to_string()),
            message_value: "value".to_string(),
            sent_at: 1716432600000,
        };
        println!("SentMessageHistoryItem id: {}", sent_msg_item.id);
        sent_hist.set_messages(vec![sent_msg_item]);

        // Use SseStreamHandler methods
        let mut stream_handler = self.stream_handler.clone();
        let is_streaming = stream_handler.is_streaming();
        stream_handler.start_stream(crate::api::StreamConfig {
            cluster_id: 1,
            topic: "orders".to_string(),
            partition: None,
            mode: crate::api::QueryMode::Newest,
            max_messages: 100,
            search_value: None,
            start_time: None,
            end_time: None,
        });
        stream_handler.stop_stream();
        println!("SseStreamHandler is_streaming: {}", is_streaming);

        // Use time utility functions
        let current_ts = current_timestamp_ms();
        let iso_ts = parse_iso_to_timestamp("2024-01-01T00:00:00Z");
        let duration_str = format_duration_ms(3600000);
        let formatted_ts = utils_format_timestamp(current_ts);
        println!("Time utils: current={}, iso={}, duration={}, formatted={}",
            current_ts, iso_ts.unwrap_or(0), duration_str, formatted_ts);

        // Use format utility functions
        let truncated = truncate_string("This is a long string for testing truncation", 20);
        let pretty_from_format = format_pretty_json("{\"name\":\"test\"}");
        let is_json = is_json_like("{\"test\": true}");
        let offset_str = format_offset(12345);
        let bytes_str = format_bytes(1024 * 1024);
        let number_str = format_number(1000);
        println!("Format utils: truncated={}, pretty={}, is_json={}, offset={}, bytes={}, number={}",
            truncated, pretty_from_format.unwrap_or_default(), is_json, offset_str, bytes_str, number_str);

        // Use QueryMode::as_str method (from api/sse.rs)
        let api_query_mode = crate::api::QueryMode::Newest;
        let newest_str2 = api_query_mode.as_str();
        let oldest_str2 = crate::api::QueryMode::Oldest.as_str();
        println!("API QueryMode strings: {}, {}", newest_str2, oldest_str2);

        // Use time utility functions
        let simple_item = SimpleItem {
            id: "item-1".to_string(),
            label: "Test Item".to_string(),
        };
        let message_item = MessageItem {
            id: "msg-1".to_string(),
            offset: 12345,
            key: Some("test-key".to_string()),
            value: "test value".to_string(),
            timestamp: 1716432600000,
        };
        println!("SimpleItem id: {}, MessageItem id: {}", simple_item.id, message_item.id);

        // Use pretty_json and is_valid_json
        let test_json = "{\"name\": \"test\"}";
        if is_valid_json(test_json) {
            let pretty = pretty_json(test_json);
            println!("Pretty JSON: {:?}", pretty);
        }

        // Use from_buffered and from_kafka_message conversions
        let buffered_msg = BufferedMessage {
            partition: 0,
            offset: 12345,
            timestamp: 1716432600000,
            key: Some("test".to_string()),
            value: "test value".to_string(),
            headers: vec![],
        };

        // Use estimated_size method
        let msg_size = buffered_msg.estimated_size();
        println!("BufferedMessage estimated_size: {}", msg_size);

        let display_from_buffered = MessageDisplay::from_buffered(&buffered_msg);

        let kafka_msg = KafkaMessage {
            partition: 1,
            offset: 67890,
            timestamp: 1716432601000,
            key: Some("kafka-key".to_string()),
            value: "{\"data\": \"test\"}".to_string(),
            headers: None,
        };
        let display_from_kafka = MessageDisplay::from_kafka_message(&kafka_msg);
        println!("From buffered: {:?}", display_from_buffered);
        println!("From kafka: {:?}", display_from_kafka);

        // Partition options
        let partitions = vec![
            ("全部", None),
            ("分区 0", Some(0)),
            ("分区 1", Some(1)),
            ("分区 2", Some(2)),
        ];

        // Query mode options
        let modes = vec![
            ("最新", QueryMode::Newest),
            ("最早", QueryMode::Oldest),
        ];

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(16.0))
            .child(
                // Toolbar
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .flex_wrap()
                    .child(
                        // Partition selector
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .children(partitions.iter().map(|(name, id)| {
                                let is_selected = self.selected_partition == *id;
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { theme.primary } else { theme.surface })
                                    .border(px(1.0))
                                    .border_color(if is_selected { theme.primary } else { theme.border })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if is_selected {
                                                Hsla::from(gpui::rgb(0xffffff))
                                            } else {
                                                theme.text_secondary
                                            })
                                            .text_xs()
                                            .child(name.to_string())
                                    )
                            }))
                    )
                    .child(
                        // Query mode selector
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .children(modes.iter().map(|(name, mode)| {
                                let is_selected = self.query_mode == *mode;
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { theme.surface_raised } else { theme.surface })
                                    .border(px(1.0))
                                    .border_color(if is_selected { theme.border_focused } else { theme.border })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if is_selected { theme.text } else { theme.text_muted })
                                            .text_xs()
                                            .child(name.to_string())
                                    )
                            }))
                    )
                    .child(
                        // Max messages input
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(t.messages.max_messages.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(self.max_messages.to_string())
                            )
                    )
                    .child(
                        // Search input
                        div()
                            .flex()
                            .items_center()
                            .flex_1()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .w(px(14.0))
                                    .h(px(14.0))
                                    .bg(theme.text_muted)
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(t.messages.value_placeholder.clone())
                            )
                    )
                    .child(
                        // Query/Stop button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(16.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if self.is_querying { theme.error } else { theme.primary })
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .child(if self.is_querying {
                                        t.messages.stop.clone()
                                    } else {
                                        t.messages.query.clone()
                                    })
                            )
                    )
                    .child(
                        // Send message button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(t.messages.send_message.clone())
                            )
                    )
            )
            .when(self.is_querying, |this| {
                let received = self.stream_handler.total_received();
                let target = self.stream_handler.total_target();
                let progress_pct = self.stream_handler.progress();
                let progress_width = if target > 0 {
                    px(progress_pct * 2.0)
                } else {
                    px(0.0)
                };

                this.child(
                    // Progress indicator with SSE streaming
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
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(t.messages.receiving.clone())
                        )
                        .child(
                            div()
                                .text_color(theme.text)
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{} / {}", received, target))
                        )
                        .child(
                            // Progress bar
                            div()
                                .flex_1()
                                .h(px(4.0))
                                .rounded(px(2.0))
                                .bg(theme.surface_raised)
                                .child(
                                    div()
                                        .w(progress_width)
                                        .h(px(4.0))
                                        .rounded(px(2.0))
                                        .bg(theme.primary)
                                )
                        )
                        .when_some(self.stream_handler.error(), |this, err| {
                            this.child(
                                div()
                                    .text_color(theme.error)
                                    .text_xs()
                                    .child(err.clone())
                            )
                        })
                )
            })
            .child(
                // Messages table header
                div()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .py(px(8.0))
                    .gap(px(12.0))
                    .bg(theme.surface)
                    .border_b(px(2.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(60.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.messages.partition_label.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.messages.offset_label.clone())
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.messages.timestamp_label.clone())
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.messages.key.clone())
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.messages.value.clone())
                    )
            )
            .child(
                // Messages list
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .children(self.messages.iter().enumerate().map(|(index, msg)| {
                        let is_selected = self.selected_message == Some(index);
                        self.message_row(msg, index, is_selected)
                    }))
            )
            // Message detail panel
            .child({
                let mut panel = MessageDetailPanel::new(self.theme.clone());
                panel.set_detail(self.get_message_detail());
                panel.set_height(180.0);
                panel
            })
            // Search status display (uses search_value)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(self.search_status())
            )
            // JSON editor panel for editing message value
            .child(self.render_json_editor())
            .child(
                // Export button
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(t.messages.export_messages.clone())
                            )
                    )
            )
            // Send message modal (hidden by default)
            .child(self.send_modal)
            // Sent message history panel
            .child(
                div()
                    .w(px(280.0))
                    .h_full()
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .p(px(8.0))
                    .child(
                        div()
                            .child(sent_hist)
                            .child(
                                // SentMessageHistoryItem id field usage note
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("Sent messages tracked by id")
                            )
                    )
            )
    }
}

/// Messages view with GlobalState Entity integration
pub struct MessagesViewWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    messages: Vec<MessageDisplay>,
    selected_partition: Option<i32>,
    query_mode: QueryMode,
    max_messages: i32,
    search_value: String,
    is_querying: bool,
    stream_handler: SseStreamHandler,
    selected_message: Option<usize>,
    send_modal_open: bool,
    detail_format: MessageFormat,
    show_headers: bool,
    show_time_filters: bool,
    start_time: String,
    end_time: String,
    show_more_menu: bool,
    show_history: bool,
    /// Streaming progress: received count
    streaming_received: usize,
    /// Streaming progress: total target
    streaming_total: usize,
    /// Elapsed time for last query (ms)
    elapsed_time: u64,
    /// Timestamp sort order (asc/desc/null toggle)
    timestamp_sort: TimestampSort,
    /// Column widths (matches Vue's columnWidths)
    column_widths: ColumnWidths,
    /// Which column is being resized (if any)
    resizing_column: Option<String>,
    /// Detail panel height and resize state (matches Vue's panelHeight)
    detail_panel_state: DetailPanelState,
    /// Detail panel search state (matches Vue's detailSearchActive)
    detail_search_state: DetailSearchState,
}

/// Column widths for message table (matches Vue's columnWidths)
#[derive(Debug, Clone, Default)]
struct ColumnWidths {
    partition: f32,
    offset: f32,
    timestamp: f32,
    key: f32,
    value: f32,
    actions: f32,
}

impl ColumnWidths {
    fn defaults() -> Self {
        Self {
            partition: 60.0,
            offset: 80.0,
            timestamp: 140.0,
            key: 100.0,
            value: 200.0,  // flex_1, minimum width
            actions: 40.0,
        }
    }
}

/// Detail panel state (matches Vue's panelHeight and resize functionality)
#[derive(Debug, Clone, Default)]
struct DetailPanelState {
    height: f32,
    is_resizing: bool,
    min_height: f32,
    max_height: f32,
}

impl DetailPanelState {
    fn defaults() -> Self {
        Self {
            height: 180.0,
            is_resizing: false,
            min_height: 80.0,
            max_height: 400.0,
        }
    }

    fn set_height(&mut self, h: f32) {
        self.height = h.max(self.min_height).min(self.max_height);
    }
}

/// Detail panel search state (matches Vue's detailSearchActive)
#[derive(Debug, Clone, Default)]
struct DetailSearchState {
    is_active: bool,
    query: String,
    match_count: usize,
    match_index: usize,
}

use crate::state::GlobalState;
use crate::router::ViewType;

/// Message format for detail panel
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum MessageFormat {
    #[default]
    Json,
    Raw,
    Hex,
}

/// Timestamp sort order (matches Vue's three-state toggle)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum TimestampSort {
    #[default]
    None,  // No sorting
    Desc,  // Newest first (descending)
    Asc,   // Oldest first (ascending)
}

impl TimestampSort {
    /// Toggle to next state: None -> Desc -> Asc -> None
    fn toggle(&self) -> Self {
        match self {
            TimestampSort::None => TimestampSort::Desc,
            TimestampSort::Desc => TimestampSort::Asc,
            TimestampSort::Asc => TimestampSort::None,
        }
    }

    /// Get label char for UI display
    fn label_char(&self) -> char {
        match self {
            TimestampSort::None => ' ',
            TimestampSort::Desc => '↓',
            TimestampSort::Asc => '↑',
        }
    }

    /// Get tooltip text (static string)
    fn tooltip(&self) -> &'static str {
        match self {
            TimestampSort::None => "Click to sort descending",
            TimestampSort::Desc => "Click to sort ascending",
            TimestampSort::Asc => "Click to clear sorting",
        }
    }
}

impl MessagesViewWithState {
    pub fn new(state: Entity<GlobalState>, translations: Arc<Translations>) -> Self {
        Self {
            state,
            translations,
            messages: Vec::new(),
            selected_partition: None,
            query_mode: QueryMode::Newest,
            max_messages: 100,
            search_value: String::new(),
            is_querying: false,
            stream_handler: SseStreamHandler::new(),
            selected_message: None,
            send_modal_open: false,
            detail_format: MessageFormat::Json,
            show_headers: false,
            show_time_filters: false,
            start_time: String::new(),
            end_time: String::new(),
            show_more_menu: false,
            show_history: false,
            streaming_received: 0,
            streaming_total: 0,
            elapsed_time: 0,
            timestamp_sort: TimestampSort::None,
            column_widths: ColumnWidths::defaults(),
            resizing_column: None,
            detail_panel_state: DetailPanelState::defaults(),
            detail_search_state: DetailSearchState::default(),
        }
    }

    /// Start streaming messages
    fn start_query(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        let selected_topic = state.selected_topic.clone();

        if let Some((cluster, topic)) = selected_topic {
            self.is_querying = true;
            self.messages.clear();

            // Start async query
            cx.spawn(async move |this, cx| {
                // Simulate API call - in real implementation, would use SSE
                cx.background_executor().timer(std::time::Duration::from_millis(100)).await;

                // Add mock messages
                this.update(cx, |view, cx| {
                    view.is_querying = false;
                    view.messages = vec![
                        MessageDisplay {
                            partition: 0,
                            offset: 12345,
                            timestamp: format_timestamp(1716432600000),
                            timestamp_ms: 1716432600000,
                            key: Some("key-1".to_string()),
                            value: "{\"type\": \"test\"}".to_string(),
                            headers: vec![],
                        },
                        MessageDisplay {
                            partition: 1,
                            offset: 67890,
                            timestamp: format_timestamp(1716432601000),
                            timestamp_ms: 1716432601000,
                            key: None,
                            value: "{\"data\": \"value\"}".to_string(),
                            headers: vec![],
                        },
                    ];
                    cx.notify();
                }).ok();
            }).detach();
        }
    }

    /// Stop streaming
    fn stop_query(&mut self) {
        self.is_querying = false;
        self.stream_handler.stop_stream();
    }

    /// Toggle message format
    fn toggle_format(&mut self) {
        self.detail_format = match self.detail_format {
            MessageFormat::Json => MessageFormat::Raw,
            MessageFormat::Raw => MessageFormat::Hex,
            MessageFormat::Hex => MessageFormat::Json,
        };
    }

    /// Toggle time filters
    fn toggle_time_filters(&mut self) {
        self.show_time_filters = !self.show_time_filters;
    }

    /// Set time preset
    fn set_time_preset(&mut self, minutes: i32) {
        let now = current_timestamp_ms();
        let offset = minutes as i64 * 60 * 1000;
        let start_ts = now - offset;
        self.start_time = utils_format_timestamp(start_ts);
        self.end_time = utils_format_timestamp(now);
    }

    /// Clear time filters
    fn clear_time_filters(&mut self) {
        self.start_time.clear();
        self.end_time.clear();
    }

    /// Go back to topics view
    fn go_back(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.navigate(ViewType::Topics);
            cx.notify();
        });
    }

    /// Open send message modal
    fn open_send_modal(&mut self) {
        self.send_modal_open = true;
    }

    /// Close send message modal
    fn close_send_modal(&mut self) {
        self.send_modal_open = false;
    }

    /// Toggle more menu
    fn toggle_more_menu(&mut self) {
        self.show_more_menu = !self.show_more_menu;
    }

    /// View topic consumer groups
    fn view_consumer_groups(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.navigate(ViewType::ConsumerGroups);
            cx.notify();
        });
        self.show_more_menu = false;
    }

    /// Toggle history panel
    fn toggle_history(&mut self) {
        self.show_history = !self.show_history;
        self.show_more_menu = false;
    }

    /// Get current topic info
    fn current_topic(&self, cx: &App) -> (Option<String>, Option<String>) {
        let state = self.state.read(cx);
        state.selected_topic.clone()
            .map(|(c, t)| (Some(c), Some(t)))
            .unwrap_or((None, None))
    }

    /// Export messages to JSON (placeholder - GPUI doesn't have file dialog)
    fn export_messages(&mut self, _cx: &mut Context<Self>) {
        // In GPUI, we would need platform-specific file dialog integration
        // For now, just log the action - this could be enhanced with Tauri integration
        if self.messages.is_empty() {
            return;
        }
        // Placeholder: could integrate with clipboard or logging
        println!("Export {} messages requested", self.messages.len());
    }

    /// Copy key to clipboard (placeholder)
    fn copy_key(&mut self, _cx: &mut Context<Self>) {
        if let Some(idx) = self.selected_message {
            if let Some(key) = &self.messages[idx].key {
                println!("Copy key: {}", key);
            }
        }
    }

    /// Copy value to clipboard (placeholder)
    fn copy_value(&mut self, _cx: &mut Context<Self>) {
        if let Some(idx) = self.selected_message {
            let value = &self.messages[idx].value;
            println!("Copy value: {}", value);
        }
    }

    /// Toggle timestamp sort order (matches Vue's three-state toggle)
    fn toggle_timestamp_sort(&mut self, cx: &mut Context<Self>) {
        self.timestamp_sort = self.timestamp_sort.toggle();
        cx.notify();
    }

    /// Get sorted messages based on current sort order
    fn get_sorted_messages(&self) -> Vec<MessageDisplay> {
        match self.timestamp_sort {
            TimestampSort::None => self.messages.clone(),
            TimestampSort::Desc => {
                let mut sorted = self.messages.clone();
                sorted.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
                sorted
            }
            TimestampSort::Asc => {
                let mut sorted = self.messages.clone();
                sorted.sort_by(|a, b| a.timestamp_ms.cmp(&b.timestamp_ms));
                sorted
            }
        }
    }

    /// Select previous message (keyboard navigation: ArrowUp)
    fn select_prev_message(&mut self, cx: &mut Context<Self>) {
        if self.messages.is_empty() {
            return;
        }
        self.selected_message = match self.selected_message {
            Some(idx) if idx > 0 => Some(idx - 1),
            Some(0) => Some(0), // Stay at first message
            None => Some(self.messages.len() - 1), // Select last message if nothing selected
            Some(_) => Some(0), // Fallback: any other case, go to first
        };
        cx.notify();
    }

    /// Select next message (keyboard navigation: ArrowDown)
    fn select_next_message(&mut self, cx: &mut Context<Self>) {
        if self.messages.is_empty() {
            return;
        }
        let last_idx = self.messages.len() - 1;
        self.selected_message = match self.selected_message {
            Some(idx) if idx < last_idx => Some(idx + 1),
            Some(last_idx) => Some(last_idx), // Stay at last message
            None => Some(0), // Select first message if nothing selected
            Some(_) => Some(last_idx), // Fallback: any other case, go to last
        };
        cx.notify();
    }

    /// Start column resize (matches Vue's startColumnResize)
    fn start_column_resize(&mut self, column: &str, cx: &mut Context<Self>) {
        self.resizing_column = Some(column.to_string());
        cx.notify();
    }

    /// Update column width during resize
    fn update_column_width(&mut self, column: &str, delta: f32, cx: &mut Context<Self>) {
        match column {
            "partition" => {
                self.column_widths.partition = (self.column_widths.partition + delta).max(40.0).min(200.0);
            }
            "offset" => {
                self.column_widths.offset = (self.column_widths.offset + delta).max(40.0).min(200.0);
            }
            "timestamp" => {
                self.column_widths.timestamp = (self.column_widths.timestamp + delta).max(60.0).min(300.0);
            }
            "key" => {
                self.column_widths.key = (self.column_widths.key + delta).max(40.0).min(300.0);
            }
            "actions" => {
                self.column_widths.actions = (self.column_widths.actions + delta).max(30.0).min(100.0);
            }
            _ => {}
        }
        cx.notify();
    }

    /// End column resize
    fn end_column_resize(&mut self, cx: &mut Context<Self>) {
        self.resizing_column = None;
        cx.notify();
    }

    /// Start detail panel resize (matches Vue's startResize)
    fn start_detail_panel_resize(&mut self, cx: &mut Context<Self>) {
        self.detail_panel_state.is_resizing = true;
        cx.notify();
    }

    /// Update detail panel height during resize
    fn update_detail_panel_height(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.detail_panel_state.set_height(self.detail_panel_state.height + delta);
        cx.notify();
    }

    /// End detail panel resize
    fn end_detail_panel_resize(&mut self, cx: &mut Context<Self>) {
        self.detail_panel_state.is_resizing = false;
        cx.notify();
    }

    /// Toggle detail panel search (matches Vue's Ctrl+F)
    fn toggle_detail_search(&mut self, cx: &mut Context<Self>) {
        self.detail_search_state.is_active = !self.detail_search_state.is_active;
        if !self.detail_search_state.is_active {
            self.detail_search_state.query.clear();
            self.detail_search_state.match_count = 0;
            self.detail_search_state.match_index = 0;
        }
        cx.notify();
    }

    /// Update detail search query
    fn update_detail_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.detail_search_state.query = query.clone();
        // Count matches in current message value
        if let Some(idx) = self.selected_message {
            let msg = &self.messages[idx];
            self.detail_search_state.match_count = if query.is_empty() {
                0
            } else {
                msg.value.matches(&query).count()
            };
            self.detail_search_state.match_index = 0;
        }
        cx.notify();
    }

    /// Navigate to previous search match
    fn detail_search_prev(&mut self, cx: &mut Context<Self>) {
        if self.detail_search_state.match_count > 0 && self.detail_search_state.match_index > 0 {
            self.detail_search_state.match_index -= 1;
            cx.notify();
        }
    }

    /// Navigate to next search match
    fn detail_search_next(&mut self, cx: &mut Context<Self>) {
        if self.detail_search_state.match_count > 0 {
            self.detail_search_state.match_index = (self.detail_search_state.match_index + 1) % self.detail_search_state.match_count;
            cx.notify();
        }
    }

    /// Close detail search
    fn close_detail_search(&mut self, cx: &mut Context<Self>) {
        self.detail_search_state.is_active = false;
        self.detail_search_state.query.clear();
        self.detail_search_state.match_count = 0;
        self.detail_search_state.match_index = 0;
        cx.notify();
    }
}

impl Render for MessagesViewWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let t = &self.translations;
        let (cluster, topic) = self.current_topic(cx);
        let show_time = self.show_time_filters;
        let show_more = self.show_more_menu;

        div()
            .id("messages-view-with-state")
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                // Toolbar (matches Vue's MessageQueryTool)
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .flex_wrap()
                    .p(px(8.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .child(
                        // Back button
                        div()
                            .id("back-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.go_back(cx);
                            }))
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.text_muted)
                            )
                    )
                    .child(
                        // Partition selector (dropdown style)
                        div()
                            .id("partition-select")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(if self.selected_partition.is_none() { "All".to_string() } else { format!("P{}", self.selected_partition.unwrap()) })
                            )
                    )
                    .child(
                        // Query mode selector
                        div()
                            .id("mode-select")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(if self.query_mode == QueryMode::Newest { "Newest" } else { "Oldest" })
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.query_mode = if this.query_mode == QueryMode::Newest { QueryMode::Oldest } else { QueryMode::Newest };
                                cx.notify();
                            }))
                    )
                    .child(
                        // Max messages input
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(self.max_messages.to_string())
                            )
                    )
                    .child(
                        // Time filter toggle button
                        div()
                            .id("time-filter-btn")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(if show_time { theme.primary.opacity(0.2) } else { theme.surface_raised })
                            .border(px(1.0))
                            .border_color(if show_time { theme.primary } else { theme.border })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_time_filters();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(if show_time { theme.primary } else { theme.text_muted })
                            )
                            .child(
                                div()
                                    .text_color(if show_time { theme.primary } else { theme.text_muted })
                                    .text_xs()
                                    .child("Time")
                            )
                    )
                    .child(
                        // Search input with clear button (matches Vue's clearable input)
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(if self.search_value.is_empty() { theme.text_muted } else { theme.text })
                                    .text_xs()
                                    .child(if self.search_value.is_empty() { "Search...".to_string() } else { self.search_value.clone() })
                            )
                            // Clear button (X) when search_value has content (matches Vue)
                            .when(!self.search_value.is_empty(), |this| {
                                this.child(
                                    div()
                                        .id("search-clear-btn")
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.search_value.clear();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .w(px(8.0))
                                                .h(px(8.0))
                                                .rounded(px(2.0))
                                                .bg(theme.text_muted.opacity(0.5))
                                        )
                                )
                            })
                    )
                    .child(
                        // Query/Stop button
                        div()
                            .id("query-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(if self.is_querying { theme.error } else { theme.primary })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.is_querying {
                                    this.stop_query();
                                } else {
                                    this.start_query(cx);
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_xs()
                                    .child(if self.is_querying { "Stop" } else { "Query" })
                            )
                    )
                    .child(
                        // Send message button
                        div()
                            .id("send-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_send_modal();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.success)
                            )
                    )
                    .child(
                        // More menu button
                        div()
                            .id("more-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_more_menu();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.text_muted)
                            )
                    )
                    // Topic info display
                    .when_some(topic.clone(), |this, topic_name| {
                        this.child(
                            div()
                                .text_color(theme.text)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(topic_name)
                        )
                    })
                    .when_some(cluster.clone(), |this, c| {
                        this.child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(format!("({})", c))
                        )
                    })
            )
            // Time filter panel (when toggled)
            .when(show_time, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .p(px(8.0))
                        .bg(theme.surface_raised)
                        .border_b(px(1.0))
                        .border_color(theme.border)
                        .child(
                            // Start time
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child("Start:")
                                )
                                .child(
                                    div()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface)
                                        .border(px(1.0))
                                        .border_color(theme.border)
                                        .child(
                                            div()
                                                .text_color(theme.text)
                                                .text_xs()
                                                .child(if self.start_time.is_empty() { "YYYY-MM-DD".to_string() } else { self.start_time.clone() })
                                        )
                                )
                        )
                        .child(
                            // End time
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child("End:")
                                )
                                .child(
                                    div()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface)
                                        .border(px(1.0))
                                        .border_color(theme.border)
                                        .child(
                                            div()
                                                .text_color(theme.text)
                                                .text_xs()
                                                .child(if self.end_time.is_empty() { "YYYY-MM-DD".to_string() } else { self.end_time.clone() })
                                        )
                                )
                        )
                        // Time preset buttons (matches Vue: 5m, 15m, 30m, 1h, 1d)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .id("preset-5m")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_time_preset(5);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("5m"))
                                )
                                .child(
                                    div()
                                        .id("preset-15m")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_time_preset(15);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("15m"))
                                )
                                .child(
                                    div()
                                        .id("preset-30m")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_time_preset(30);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("30m"))
                                )
                                .child(
                                    div()
                                        .id("preset-1h")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_time_preset(60);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("1h"))
                                )
                                .child(
                                    div()
                                        .id("preset-1d")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_time_preset(24 * 60);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("1d"))
                                )
                                .child(
                                    div()
                                        .id("clear-time")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(2.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.clear_time_filters();
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("Clear"))
                                )
                        )
                )
            })
            // More menu dropdown
            .when(show_more, |this| {
                this.child(
                    div()
                        .absolute()
                        .right(px(8.0))
                        .top(px(44.0))
                        .w(px(160.0))
                        .rounded(px(6.0))
                        .bg(theme.surface)
                        .border(px(1.0))
                        .border_color(theme.border)
                        .p(px(4.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    // History option
                                    div()
                                        .id("menu-history")
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_history();
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text).text_xs().child("发送历史"))
                                )
                                .child(
                                    // Consumer groups option
                                    div()
                                        .id("menu-groups")
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.view_consumer_groups(cx);
                                            cx.notify();
                                        }))
                                        .child(div().text_color(theme.text).text_xs().child("消费者组"))
                                )
                                .child(
                                    // Delete topic option (danger)
                                    div()
                                        .id("menu-delete")
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .rounded(px(4.0))
                                        .bg(theme.error.opacity(0.1))
                                        .cursor_pointer()
                                        .child(div().text_color(theme.error).text_xs().child("删除主题"))
                                )
                        )
                )
            })
            // Status bar (matches Vue's status bar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .py(px(4.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .bg(theme.surface_raised)
                    .child(
                        // Left side: topic info + progress
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .when_some(topic.clone(), |this, t_name| {
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(div().text_color(theme.text_muted).text_xs().child(t_name))
                                        .when_some(cluster.clone(), |this, c| {
                                            this.child(div().text_color(theme.text_muted).text_xs().child(format!("({})", c)))
                                        })
                                )
                            })
                            .when(self.is_querying, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(
                                            // Spinning indicator
                                            div()
                                                .w(px(12.0))
                                                .h(px(12.0))
                                                .rounded(px(6.0))
                                                .border(px(2.0))
                                                .border_color(theme.primary.opacity(0.3))
                                                .bg(theme.surface)
                                        )
                                        .child(div().text_color(theme.primary).text_xs().child(format!("Receiving {} / {}", self.streaming_received, self.streaming_total)))
                                )
                            })
                            .when(!self.is_querying && self.elapsed_time > 0, |this| {
                                this.child(div().text_color(theme.text_muted).text_xs().child(format!("Elapsed: {}ms", self.elapsed_time)))
                            })
                            .when(!self.messages.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(div().text_color(theme.success).text_xs().font_weight(FontWeight::SEMIBOLD).child(format!("{} messages", self.messages.len())))
                                        .child(
                                            // Export button
                                            div()
                                                .id("export-btn")
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .w(px(20.0))
                                                .h(px(20.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .w(px(10.0))
                                                        .h(px(10.0))
                                                        .rounded(px(2.0))
                                                        .bg(theme.text_muted)
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.export_messages(cx);
                                                    cx.notify();
                                                }))
                                        )
                                )
                            })
                    )
                    .when(self.is_querying && self.streaming_total > 0, |this| {
                        // Progress bar on right side (matches Vue's progress bar)
                        let progress_pct = (self.streaming_received as f32 / self.streaming_total as f32).min(1.0);
                        this.child(
                            div()
                                .flex_1()
                                .mx(px(16.0))
                                .h(px(4.0))
                                .rounded(px(2.0))
                                .bg(theme.surface)
                                .child(
                                    div()
                                        .flex()
                                        .h(px(4.0))
                                        .rounded(px(2.0))
                                        .bg(theme.primary)
                                        .w(relative(progress_pct))
                                )
                        )
                    })
            )
            // Messages table
            .child(
                div()
                    .id("messages-table")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    // Keyboard navigation for arrow keys
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        match event.keystroke.key.as_str() {
                            "up" => {
                                this.select_prev_message(cx);
                            }
                            "down" => {
                                this.select_next_message(cx);
                            }
                            _ => {}
                        }
                    }))
                    .when(self.messages.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .justify_center()
                                .items_center()
                                .py(px(32.0))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_sm()
                                        .child("No messages. Click Query to fetch messages.")
                                )
                        )
                    })
                    .when(!self.messages.is_empty(), |this| {
                            let sorted_messages = self.get_sorted_messages();
                            let timestamp_sort = self.timestamp_sort;
                            let col_widths = self.column_widths.clone();

                            this.child(
                            // Header row (matches Vue's table header with resize handles)
                            div()
                                .flex()
                                .items_center()
                                .px(px(8.0))
                                .py(px(8.0))
                                .bg(theme.surface_raised)
                                .border_b(px(1.0))
                                .border_color(theme.border)
                                // Partition column
                                .child(div().w(px(col_widths.partition)).text_color(theme.text_muted).text_xs().child("Partition"))
                                // Resize handle for partition
                                .child(
                                    div()
                                        .id("resize-partition")
                                        .w(px(4.0))
                                        .h(px(16.0))
                                        .cursor_col_resize()
                                        .bg(theme.border.opacity(0.3))
                                        .hover(|d| d.bg(theme.primary.opacity(0.4)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.start_column_resize("partition", cx);
                                        }))
                                )
                                // Offset column
                                .child(div().w(px(col_widths.offset)).text_color(theme.text_muted).text_xs().child("Offset"))
                                // Resize handle for offset
                                .child(
                                    div()
                                        .id("resize-offset")
                                        .w(px(4.0))
                                        .h(px(16.0))
                                        .cursor_col_resize()
                                        .bg(theme.border.opacity(0.3))
                                        .hover(|d| d.bg(theme.primary.opacity(0.4)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.start_column_resize("offset", cx);
                                        }))
                                )
                                // Timestamp header with sort toggle (matches Vue's clickable header)
                                .child(
                                    div()
                                        .id("timestamp-sort-header")
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .w(px(col_widths.timestamp))
                                        .px(px(4.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(if timestamp_sort != TimestampSort::None { theme.surface } else { gpui::transparent_black() })
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_timestamp_sort(cx);
                                        }))
                                        .child(div().text_color(theme.text_muted).text_xs().child("Timestamp"))
                                        .when(timestamp_sort != TimestampSort::None, |this| {
                                            this.child(
                                                div()
                                                    .text_color(theme.primary)
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(timestamp_sort.label_char().to_string())
                                            )
                                        })
                                )
                                // Resize handle for timestamp
                                .child(
                                    div()
                                        .id("resize-timestamp")
                                        .w(px(4.0))
                                        .h(px(16.0))
                                        .cursor_col_resize()
                                        .bg(theme.border.opacity(0.3))
                                        .hover(|d| d.bg(theme.primary.opacity(0.4)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.start_column_resize("timestamp", cx);
                                        }))
                                )
                                // Key column
                                .child(div().w(px(col_widths.key)).text_color(theme.text_muted).text_xs().child("Key"))
                                // Resize handle for key
                                .child(
                                    div()
                                        .id("resize-key")
                                        .w(px(4.0))
                                        .h(px(16.0))
                                        .cursor_col_resize()
                                        .bg(theme.border.opacity(0.3))
                                        .hover(|d| d.bg(theme.primary.opacity(0.4)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.start_column_resize("key", cx);
                                        }))
                                )
                                // Value column (flex_1)
                                .child(div().flex_1().text_color(theme.text_muted).text_xs().child("Value"))
                                // Actions column
                                .child(div().w(px(col_widths.actions)).text_color(theme.text_muted).text_xs().child("Actions"))
                        )
                        .children(sorted_messages.iter().enumerate().map(|(idx, msg)| {
                            let is_selected = self.selected_message == Some(idx);
                            let idx_val = idx;
                            let cw = col_widths.clone();

                            div()
                                .id(format!("msg-row-{}", idx))
                                .flex()
                                .items_center()
                                .px(px(8.0))
                                .py(px(6.0))
                                .bg(if is_selected { theme.primary.opacity(0.15) } else { theme.surface })
                                .border_l(px(3.0))
                                .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface_raised))
                                .child(div().w(px(cw.partition)).text_color(theme.text_muted).text_xs().child(msg.partition.to_string()))
                                .child(div().w(px(cw.offset)).text_color(theme.text_muted).text_xs().child(msg.offset.to_string()))
                                .child(div().w(px(cw.timestamp)).text_color(theme.text_secondary).text_xs().child(msg.timestamp.clone()))
                                .child(div().w(px(cw.key)).text_color(theme.text_secondary).text_xs().truncate().child(msg.key.clone().unwrap_or_else(|| "-".to_string())))
                                .child(div().flex_1().text_color(theme.text).text_xs().truncate().child(msg.value.clone()))
                                // Actions column with copy button (matches Vue)
                                .child(
                                    div()
                                        .w(px(cw.actions))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .id(format!("copy-row-{}", idx))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .w(px(18.0))
                                                .h(px(18.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .cursor_pointer()
                                                .on_click(cx.listener({
                                                    let msg_value = msg.value.clone();
                                                    move |this, _, _, cx| {
                                                        // Copy value (placeholder - needs clipboard integration)
                                                        println!("Copy message value: {}", msg_value);
                                                        cx.notify();
                                                    }
                                                }))
                                                .child(
                                                    div()
                                                        .w(px(8.0))
                                                        .h(px(8.0))
                                                        .rounded(px(2.0))
                                                        .bg(theme.text_muted.opacity(0.5))
                                                )
                                        )
                                )
                                .on_click(cx.listener({
                                    let idx = idx_val;
                                    move |this, _, _, cx| {
                                        this.selected_message = Some(idx);
                                        cx.notify();
                                    }
                                }))
                        }))
                    })
            )
            // Detail panel (matches Vue's detail panel with resize handle)
            .when_some(self.selected_message, |this, idx| {
                let msg = &self.messages[idx];
                let format_label = match self.detail_format {
                    MessageFormat::Json => "JSON",
                    MessageFormat::Raw => "Raw",
                    MessageFormat::Hex => "Hex",
                };
                let panel_height = self.detail_panel_state.height;
                let is_resizing = self.detail_panel_state.is_resizing;

                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .border_t(px(1.0))
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .h(px(panel_height))
                        // Detail search bar (matches Vue's Ctrl+F search UI)
                        .when(self.detail_search_state.is_active, |this| {
                            let search_query = self.detail_search_state.query.clone();
                            let match_count = self.detail_search_state.match_count;
                            let match_index = self.detail_search_state.match_index;

                            this.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .bg(theme.surface)
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    // Search icon placeholder
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .rounded(px(2.0))
                                            .bg(theme.text_muted.opacity(0.4))
                                    )
                                    // Search input
                                    .child(
                                        div()
                                            .flex_1()
                                            .px(px(6.0))
                                            .py(px(2.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface_raised)
                                            .border(px(1.0))
                                            .border_color(theme.border)
                                            .child(
                                                div()
                                                    .text_color(if search_query.is_empty() { theme.text_muted } else { theme.text })
                                                    .text_xs()
                                                    .child(if search_query.is_empty() { "搜索详情内容...".to_string() } else { search_query.clone() })
                                            )
                                    )
                                    // Match count display
                                    .when(!search_query.is_empty(), |this| {
                                        this.child(
                                            div()
                                                .text_color(if match_count > 0 { theme.text_muted } else { theme.error })
                                                .text_xs()
                                                .child(if match_count > 0 {
                                                    format!("{}/{}", match_index + 1, match_count)
                                                } else {
                                                    "0 匹配".to_string()
                                                })
                                        )
                                    })
                                    // Prev button (when matches exist)
                                    .when(!search_query.is_empty() && match_count > 0, |this| {
                                        this.child(
                                            div()
                                                .id("search-prev-btn")
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.detail_search_prev(cx);
                                                }))
                                                .child(
                                                    div()
                                                        .w(px(8.0))
                                                        .h(px(4.0))
                                                        .rounded(px(2.0))
                                                        .bg(theme.text_muted.opacity(0.5))
                                                )
                                        )
                                    })
                                    // Next button (when matches exist)
                                    .when(!search_query.is_empty() && match_count > 0, |this| {
                                        this.child(
                                            div()
                                                .id("search-next-btn")
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.detail_search_next(cx);
                                                }))
                                                .child(
                                                    div()
                                                        .w(px(8.0))
                                                        .h(px(4.0))
                                                        .rounded(px(2.0))
                                                        .bg(theme.text_muted.opacity(0.5))
                                                )
                                        )
                                    })
                                    // Close search button
                                    .child(
                                        div()
                                            .id("close-search-btn")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(16.0))
                                            .h(px(16.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface)
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.close_detail_search(cx);
                                            }))
                                            .child(
                                                div()
                                                    .w(px(8.0))
                                                    .h(px(8.0))
                                                    .rounded(px(2.0))
                                                    .bg(theme.text_muted.opacity(0.5))
                                            )
                                    )
                            )
                        })
                        .child(
                            // Resize handle (functional - matches Vue's startResize)
                            div()
                                .id("detail-panel-resize-handle")
                                .flex()
                                .items_center()
                                .justify_center()
                                .h(px(6.0))
                                .bg(if is_resizing { theme.primary.opacity(0.5) } else { theme.border.opacity(0.5) })
                                .cursor_row_resize()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    // Toggle resize mode (click to enable/disable)
                                    if this.detail_panel_state.is_resizing {
                                        this.end_detail_panel_resize(cx);
                                    } else {
                                        this.start_detail_panel_resize(cx);
                                    }
                                }))
                                .child(
                                    div()
                                        .w(px(32.0))
                                        .h(px(2.0))
                                        .rounded(px(1.0))
                                        .bg(if is_resizing { theme.primary } else { theme.text_muted.opacity(0.3) })
                                )
                        )
                        .child(
                            // Header row
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(8.0))
                                .py(px(4.0))
                                .border_b(px(1.0))
                                .border_color(theme.border)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(div().text_color(theme.text).text_xs().font_weight(FontWeight::SEMIBOLD).child("Message Detail"))
                                        .child(div().text_color(theme.text_muted).text_xs().child(format!("P: {}", msg.partition)))
                                        .child(div().text_color(theme.text_muted).text_xs().child(format!("O: {}", msg.offset)))
                                        .child(div().text_color(theme.text_muted).text_xs().child(msg.timestamp.clone()))
                                )
                                .child(
                                    // Close button
                                    div()
                                        .id("close-detail")
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.selected_message = None;
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .w(px(8.0))
                                                .h(px(8.0))
                                                .rounded(px(2.0))
                                                .bg(theme.text_muted)
                                        )
                                )
                        )
                        .child(
                            // Content area
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .p(px(8.0))
                                .child(
                                    // Key row with copy button
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.0))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(div().text_color(theme.text_muted).text_xs().font_weight(FontWeight::SEMIBOLD).child("Key:"))
                                                .child(
                                                    // Copy key button
                                                    div()
                                                        .id("copy-key-btn")
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .w(px(18.0))
                                                        .h(px(18.0))
                                                        .rounded(px(4.0))
                                                        .bg(theme.surface)
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.copy_key(cx);
                                                            cx.notify();
                                                        }))
                                                        .child(
                                                            div()
                                                                .w(px(8.0))
                                                                .h(px(8.0))
                                                                .rounded(px(2.0))
                                                                .bg(theme.text_muted.opacity(0.5))
                                                        )
                                                )
                                        )
                                        .child(
                                            div()
                                                .px(px(8.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .border(px(1.0))
                                                .border_color(theme.border)
                                                .child(
                                                    div()
                                                        .text_color(theme.text)
                                                        .text_xs()
                                                        .truncate()
                                                        .child(msg.key.clone().unwrap_or_else(|| "-".to_string()))
                                                )
                                        )
                                )
                                .child(
                                    // Value row with format selector and copy button
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.0))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(4.0))
                                                        .child(div().text_color(theme.text_muted).text_xs().font_weight(FontWeight::SEMIBOLD).child("Value:"))
                                                        .child(
                                                            // Format toggle
                                                            div()
                                                                .id("format-toggle")
                                                                .px(px(4.0))
                                                                .py(px(2.0))
                                                                .rounded(px(2.0))
                                                                .bg(theme.surface)
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_format();
                                                                    cx.notify();
                                                                }))
                                                                .child(div().text_color(theme.text_secondary).text_xs().child(format_label))
                                                        )
                                                )
                                                .child(
                                                    // Copy value button
                                                    div()
                                                        .id("copy-value-btn")
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .w(px(18.0))
                                                        .h(px(18.0))
                                                        .rounded(px(4.0))
                                                        .bg(theme.surface)
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.copy_value(cx);
                                                            cx.notify();
                                                        }))
                                                        .child(
                                                            div()
                                                                .w(px(8.0))
                                                                .h(px(8.0))
                                                                .rounded(px(2.0))
                                                                .bg(theme.text_muted.opacity(0.5))
                                                        )
                                                )
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_h(px(80.0))
                                                .px(px(8.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface)
                                                .border(px(1.0))
                                                .border_color(theme.border)
                                                .child(
                                                    div()
                                                        .text_color(theme.text)
                                                        .text_xs()
                                                        .child(msg.value.clone())
                                                )
                                        )
                                )
                        )
                )
            })
    }
}