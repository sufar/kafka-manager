//! Topic Context Menu
//!
//! Context menu for topic nodes in the tree navigator.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Actions available from topic context menu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopicAction {
    ViewMessages,
    ViewDetails,
    ViewPartitions,
    SendMessage,
    ExportData,
    DeleteTopic,
}

/// Topic context menu
#[derive(Clone)]
pub struct TopicContextMenu {
    theme: Theme,
    topic_name: String,
    cluster_name: String,
    position: Point<Pixels>,
}

impl TopicContextMenu {
    /// Create new topic context menu
    pub fn new(
        theme: Theme,
        topic_name: String,
        cluster_name: String,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            theme,
            topic_name,
            cluster_name,
            position,
        }
    }

    /// Get topic name
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get cluster name
    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }

    /// Render menu item
    fn render_item(&self, label: String) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(label)
            )
    }

    /// Render danger item
    fn render_danger_item(&self, label: String) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|d| d.bg(theme.error.opacity(0.1)))
            .child(
                div()
                    .text_color(theme.error)
                    .text_sm()
                    .child(label)
            )
    }

    /// Render separator
    fn render_separator(&self) -> Div {
        let theme = &self.theme;

        div()
            .w_full()
            .h(px(1.0))
            .my(px(4.0))
            .bg(theme.border)
    }
}

impl IntoElement for TopicContextMenu {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .absolute()
            .top(self.position.y)
            .left(self.position.x)
            .w(px(180.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .p(px(4.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        // Menu title
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .border_b(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.topic_name.clone())
                            )
                    )
                    .child(
                        // View actions
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("查看消息".to_string()))
                            .child(self.render_item("查看详情".to_string()))
                            .child(self.render_item("查看分区".to_string()))
                    )
                    .child(self.render_separator())
                    .child(
                        // Message actions
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("发送消息".to_string()))
                            .child(self.render_item("导出数据".to_string()))
                    )
                    .child(self.render_separator())
                    .child(
                        // Delete action
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_danger_item("删除 Topic".to_string()))
                    )
            )
    }
}