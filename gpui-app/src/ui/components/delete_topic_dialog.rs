//! Delete Topic Dialog
//!
//! Confirmation dialog for deleting Kafka topics.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Delete topic dialog
#[derive(Clone)]
pub struct DeleteTopicDialog {
    theme: Theme,
    cluster_name: String,
    topic_name: String,
    is_open: bool,
    deleting: bool,
}

impl DeleteTopicDialog {
    /// Create new dialog
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            cluster_name: String::new(),
            topic_name: String::new(),
            is_open: false,
            deleting: false,
        }
    }

    /// Open the dialog with topic info
    pub fn open(&mut self, cluster_name: String, topic_name: String) {
        self.cluster_name = cluster_name;
        self.topic_name = topic_name;
        self.is_open = true;
        self.deleting = false;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Check if open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Get topic name
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get cluster name
    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }
}

impl IntoElement for DeleteTopicDialog {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if !self.is_open {
            return div();
        }

        let theme = &self.theme;

        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.8))
            .child(
                // Dialog container
                div()
                    .flex()
                    .flex_col()
                    .w(px(360.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .p(px(16.0))
                    .child(
                        // Header with warning icon
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .pb(px(12.0))
                            .child(
                                // Warning icon placeholder
                                div()
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded(px(10.0))
                                    .bg(theme.error.opacity(0.2))
                                    .child(
                                        div()
                                            .w(px(18.0))
                                            .h(px(18.0))
                                            .bg(theme.error.opacity(0.5))
                                    )
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("删除 Topic")
                            )
                    )
                    .child(
                        // Warning message
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .py(px(12.0))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .child(format!(
                                        "确定要删除 Topic '{}' 吗？",
                                        self.topic_name
                                    ))
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(format!("集群: {}", self.cluster_name))
                            )
                            .child(
                                // Warning box
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.error.opacity(0.1))
                                    .border(px(1.0))
                                    .border_color(theme.error.opacity(0.3))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .bg(theme.error.opacity(0.5))
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.error)
                                            .text_xs()
                                            .child("此操作不可撤销，Topic 中的所有消息将永久丢失")
                                    )
                            )
                    )
                    .child(
                        // Footer
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .pt(px(12.0))
                            .border_t(px(1.0))
                            .border_color(theme.border)
                            .child(
                                // Cancel button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(16.0))
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
                                            .child("取消")
                                    )
                            )
                            .child(
                                // Delete button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.error)
                                    .cursor_pointer()
                                    .when(self.deleting, |d| d.opacity(0.5))
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_sm()
                                            .child(if self.deleting { "删除中..." } else { "确认删除" })
                                    )
                            )
                    )
            )
    }
}