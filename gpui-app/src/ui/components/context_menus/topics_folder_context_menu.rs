//! Topics Folder Context Menu
//!
//! Context menu for topics folder node in the tree navigator.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Actions available from topics folder context menu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopicsFolderAction {
    RefreshTopics,
    CreateTopic,
    ViewAllTopics,
}

/// Topics folder context menu
pub struct TopicsFolderContextMenu {
    theme: Theme,
    cluster_name: String,
    position: Point<Pixels>,
}

impl TopicsFolderContextMenu {
    /// Create new topics folder context menu
    pub fn new(
        theme: Theme,
        cluster_name: String,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            theme,
            cluster_name,
            position,
        }
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

impl IntoElement for TopicsFolderContextMenu {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let title = format!("Topics - {}", self.cluster_name);

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
                            .child(self.render_item("刷新 Topics".to_string()))
                            .child(self.render_item("创建 Topic".to_string()))
                            .child(self.render_item("查看所有 Topics".to_string()))
                    )
            )
    }
}