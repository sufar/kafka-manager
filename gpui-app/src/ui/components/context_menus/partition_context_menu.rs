//! Partition Context Menu
//!
//! Context menu for partition nodes in the tree navigator.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Actions available from partition context menu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PartitionAction {
    ViewMessages,
    SendMessage,
}

/// Partition context menu
#[derive(Clone)]
pub struct PartitionContextMenu {
    theme: Theme,
    topic_name: String,
    cluster_name: String,
    partition_id: i32,
    position: Point<Pixels>,
}

impl PartitionContextMenu {
    /// Create new partition context menu
    pub fn new(
        theme: Theme,
        topic_name: String,
        cluster_name: String,
        partition_id: i32,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            theme,
            topic_name,
            cluster_name,
            partition_id,
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
}

impl IntoElement for PartitionContextMenu {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let title = format!("{} #{}", self.cluster_name, self.partition_id);

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
                                    .child(title)
                            )
                    )
                    .child(
                        // Menu items
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("查看消息".to_string()))
                            .child(self.render_item("发送消息".to_string()))
                    )
            )
    }
}