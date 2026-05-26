//! Topic History Component
//!
//! Browsing history for topics.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Topic history item
#[derive(Debug, Clone)]
pub struct TopicHistoryItem {
    pub id: i32,
    pub cluster_id: String,
    pub topic_name: String,
    pub viewed_at: i64,
}

impl TopicHistoryItem {
    /// Get id
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// Topic history panel
#[derive(Clone)]
pub struct TopicHistory {
    theme: Theme,
    histories: Vec<TopicHistoryItem>,
    search_query: String,
    loading: bool,
}

impl TopicHistory {
    /// Create new topic history
    pub fn new(theme: Theme) -> Self {
        // Mock data
        let histories = vec![
            TopicHistoryItem {
                id: 1,
                cluster_id: "Production".to_string(),
                topic_name: "orders".to_string(),
                viewed_at: 1716124800,
            },
            TopicHistoryItem {
                id: 2,
                cluster_id: "Production".to_string(),
                topic_name: "payments".to_string(),
                viewed_at: 1716121200,
            },
            TopicHistoryItem {
                id: 3,
                cluster_id: "Development".to_string(),
                topic_name: "test-topic".to_string(),
                viewed_at: 1716117600,
            },
        ];

        Self {
            theme,
            histories,
            search_query: String::new(),
            loading: false,
        }
    }

    /// Set histories
    pub fn set_histories(&mut self, histories: Vec<TopicHistoryItem>) {
        self.histories = histories;
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Get filtered histories
    fn filtered_histories(&self) -> Vec<&TopicHistoryItem> {
        if self.search_query.is_empty() {
            self.histories.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.histories
                .iter()
                .filter(|h| h.topic_name.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Format time ago
    fn format_time_ago(&self, viewed_at: i64) -> String {
        // Simplified time formatting
        let now = 1716128400; // Mock current time
        let diff = now - viewed_at;
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

    /// Render history item
    fn render_history_item(&self, item: &TopicHistoryItem) -> Div {
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
                    .gap(px(8.0))
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
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("({})", item.cluster_id))
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
                            .child(self.format_time_ago(item.viewed_at))
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

impl IntoElement for TopicHistory {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let filtered = self.filtered_histories();

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
                                // Clock icon placeholder
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
                                    .child("浏览历史")
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
                                        .child("暂无浏览历史")
                                )
                                .child(
                                    div()
                                        .mt(px(4.0))
                                        .text_color(theme.text_muted.opacity(0.5))
                                        .text_xs()
                                        .child("浏览 Topic 时会自动记录到这里")
                                )
                        )
                    })
                    .when(!self.loading && !filtered.is_empty(), |d| {
                        d.children(filtered.iter().map(|item| {
                            self.render_history_item(item)
                        }))
                    })
            )
    }
}