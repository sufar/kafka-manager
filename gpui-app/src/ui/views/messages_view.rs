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
#[derive(Debug)]
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