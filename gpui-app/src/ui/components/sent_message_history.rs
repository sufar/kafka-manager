//! Sent Message History Component
//!
//! History of sent messages.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Sent message history item
#[derive(Debug, Clone)]
pub struct SentMessageHistoryItem {
    pub id: i32,
    pub cluster_id: String,
    pub topic_name: String,
    pub partition: i32,
    pub message_key: Option<String>,
    pub message_value: String,
    pub sent_at: i64,
}

/// Sent message history panel
#[derive(Clone)]
pub struct SentMessageHistory {
    theme: Theme,
    messages: Vec<SentMessageHistoryItem>,
    search_query: String,
    loading: bool,
}

impl SentMessageHistory {
    /// Create new sent message history
    pub fn new(theme: Theme) -> Self {
        // Mock data
        let messages = vec![
            SentMessageHistoryItem {
                id: 1,
                cluster_id: "Production".to_string(),
                topic_name: "orders".to_string(),
                partition: 0,
                message_key: Some("order-123".to_string()),
                message_value: "{\"order_id\": 123, \"status\": \"created\"}".to_string(),
                sent_at: 1716124800,
            },
            SentMessageHistoryItem {
                id: 2,
                cluster_id: "Production".to_string(),
                topic_name: "payments".to_string(),
                partition: 1,
                message_key: None,
                message_value: "{\"payment_id\": 456}".to_string(),
                sent_at: 1716121200,
            },
        ];

        Self {
            theme,
            messages,
            search_query: String::new(),
            loading: false,
        }
    }

    /// Set messages
    pub fn set_messages(&mut self, messages: Vec<SentMessageHistoryItem>) {
        self.messages = messages;
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Get filtered messages
    fn filtered_messages(&self) -> Vec<&SentMessageHistoryItem> {
        if self.search_query.is_empty() {
            self.messages.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.messages
                .iter()
                .filter(|m| m.topic_name.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Format time ago
    fn format_time_ago(&self, sent_at: i64) -> String {
        let now = 1716128400;
        let diff = now - sent_at;
        let minutes = diff / 60;
        let hours = diff / 3600;
        let days = diff / 86400;

        if minutes < 1 {
            "刚刚".to_string()
        } else if minutes < 60 {
            format!("{}分钟前", minutes)
        } else if hours < 24 {
            format!("{}小时前", hours)
        } else if days < 7 {
            format!("{}天前", days)
        } else {
            "更早".to_string()
        }
    }

    /// Truncate value for display
    fn truncate_value(&self, value: &str) -> String {
        if value.len() > 50 {
            format!("{}...", &value[..50])
        } else {
            value.to_string()
        }
    }

    /// Render message item
    fn render_message_item(&self, item: &SentMessageHistoryItem) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|d| d.bg(theme.primary.opacity(0.1)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        // Icon placeholder
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.1))
                    )
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(item.topic_name.clone())
                    )
                    .child(
                        div()
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(item.cluster_id.clone())
                            )
                    )
                    .child(
                        // Partition badge
                        div()
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.2))
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child(format!("P{}", item.partition))
                            )
                    )
                    .when(item.message_key.is_some(), |d| {
                        d.child(
                            div()
                                .text_color(theme.text_muted.opacity(0.7))
                                .text_xs()
                                .child(format!("K: {}", item.message_key.as_ref().unwrap_or(&String::new())))
                        )
                    })
                    .child(
                        div()
                            .text_color(theme.text_muted.opacity(0.7))
                            .text_xs()
                            .child(format!("V: {}", self.truncate_value(&item.message_value)))
                    )
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(self.format_time_ago(item.sent_at))
                    )
                    .child(
                        // Delete button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.error.opacity(0.1)))
                            .child(
                                div()
                                    .w(px(8.0))
                                    .h(px(8.0))
                                    .bg(theme.error.opacity(0.5))
                            )
                    )
            )
    }
}

impl IntoElement for SentMessageHistory {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let filtered = self.filtered_messages();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                // Send icon placeholder
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(8.0))
                                    .bg(theme.text_muted.opacity(0.3))
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("发送历史")
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                // Clear button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.error.opacity(0.1)))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .bg(theme.error.opacity(0.3))
                                    )
                            )
                            .child(
                                // Refresh button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .bg(theme.text_muted.opacity(0.5))
                                    )
                            )
                            .child(
                                // Close button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .bg(theme.text_muted.opacity(0.5))
                                    )
                            )
                    )
            )
            .child(
                // Search box
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(theme.text_muted.opacity(0.3))
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(if self.search_query.is_empty() {
                                "搜索 Topic...".to_string()
                            } else {
                                self.search_query.clone()
                            })
                    )
            )
            .child(
                // Content
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .when(self.loading, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .py(px(32.0))
                                .child(
                                    div()
                                        .w(px(24.0))
                                        .h(px(24.0))
                                        .rounded(px(12.0))
                                        .bg(theme.primary.opacity(0.3))
                                )
                        )
                    })
                    .when(!self.loading && filtered.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py(px(32.0))
                                .child(
                                    div()
                                        .w(px(40.0))
                                        .h(px(40.0))
                                        .rounded(px(20.0))
                                        .bg(theme.text_muted.opacity(0.2))
                                )
                                .child(
                                    div()
                                        .mt(px(8.0))
                                        .text_color(theme.text_muted)
                                        .text_sm()
                                        .child("暂无发送历史")
                                )
                                .child(
                                    div()
                                        .mt(px(4.0))
                                        .text_color(theme.text_muted.opacity(0.5))
                                        .text_xs()
                                        .child("发送消息时会自动记录到这里")
                                )
                        )
                    })
                    .when(!self.loading && !filtered.is_empty(), |d| {
                        d.children(filtered.iter().map(|item| {
                            self.render_message_item(item)
                        }))
                    })
            )
    }
}