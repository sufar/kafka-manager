//! Topic Context Menu
//!
//! Context menu for topic nodes in the tree navigator.
//! Integrates with GlobalState Entity for real actions.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::state::GlobalState;
use crate::i18n::Translations;
use crate::router::ViewType;
use std::sync::Arc;

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

impl TopicAction {
    pub fn label(&self, t: &Translations) -> String {
        match self {
            TopicAction::ViewMessages => t.messages.title.clone(),
            TopicAction::ViewDetails => "View Details".to_string(),
            TopicAction::ViewPartitions => "View Partitions".to_string(),
            TopicAction::SendMessage => "Send Message".to_string(),
            TopicAction::ExportData => "Export Data".to_string(),
            TopicAction::DeleteTopic => t.topics.delete_topic.clone(),
        }
    }
}

/// Topic context menu with GlobalState integration
pub struct TopicContextMenuWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    topic_name: String,
    cluster_name: String,
    position: Point<Pixels>,
    visible: bool,
}

impl TopicContextMenuWithState {
    pub fn new(
        state: Entity<GlobalState>,
        translations: Arc<Translations>,
        topic_name: String,
        cluster_name: String,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            state,
            translations,
            topic_name,
            cluster_name,
            position,
            visible: true,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn handle_action(&mut self, action: TopicAction, cx: &mut Context<Self>) {
        let cluster_name = self.cluster_name.clone();
        let topic_name = self.topic_name.clone();

        match action {
            TopicAction::ViewMessages => {
                self.state.update(cx, |state, cx| {
                    state.navigate_to_messages(&cluster_name, &topic_name);
                    cx.notify();
                });
            }
            TopicAction::ViewDetails => {
                self.state.update(cx, |state, cx| {
                    state.select_topic(cluster_name.clone(), topic_name.clone());
                    state.navigate(ViewType::Topics);
                    cx.notify();
                });
            }
            TopicAction::SendMessage => {
                // Could open a modal for sending messages
                println!("Send message to topic: {} in cluster: {}", topic_name, cluster_name);
            }
            TopicAction::ExportData => {
                println!("Export data from topic: {} in cluster: {}", topic_name, cluster_name);
            }
            TopicAction::DeleteTopic => {
                println!("Delete topic: {} in cluster: {}", topic_name, cluster_name);
            }
            _ => {}
        }
        self.visible = false;
    }
}

impl Render for TopicContextMenuWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("topic-context-menu-hidden");
        }

        let theme = self.state.read(cx).theme.clone();
        let t = &self.translations;

        div()
            .id("topic-context-menu-state")
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
                    // Title
                    .child(
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
                    // View actions
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("topic-action-view-messages")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(TopicAction::ViewMessages.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::ViewMessages, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("topic-action-view-details")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(TopicAction::ViewDetails.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::ViewDetails, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("topic-action-view-partitions")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(TopicAction::ViewPartitions.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::ViewPartitions, cx);
                                    }))
                            )
                    )
                    // Separator
                    .child(div().w_full().h(px(1.0)).my(px(4.0)).bg(theme.border))
                    // Message actions
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("topic-action-send-message")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(TopicAction::SendMessage.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::SendMessage, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("topic-action-export-data")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(TopicAction::ExportData.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::ExportData, cx);
                                    }))
                            )
                    )
                    // Separator
                    .child(div().w_full().h(px(1.0)).my(px(4.0)).bg(theme.border))
                    // Delete action
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(
                                div()
                                    .id("topic-action-delete")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.error.opacity(0.1)))
                                    .child(div().text_color(theme.error).text_sm().child(TopicAction::DeleteTopic.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(TopicAction::DeleteTopic, cx);
                                    }))
                            )
                    )
            )
    }
}

/// Legacy topic context menu (static version)
#[derive(Clone)]
pub struct TopicContextMenu {
    theme: Theme,
    topic_name: String,
    cluster_name: String,
    position: Point<Pixels>,
}

impl TopicContextMenu {
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

    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
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
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("查看消息"))
                            .child(self.render_item("查看详情"))
                            .child(self.render_item("查看分区"))
                    )
                    .child(self.render_separator())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("发送消息"))
                            .child(self.render_item("导出数据"))
                    )
                    .child(self.render_separator())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_danger_item("删除 Topic"))
                    )
            )
    }
}

impl TopicContextMenu {
    fn render_item(&self, label: &str) -> Div {
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
                    .child(label.to_string())
            )
    }

    fn render_danger_item(&self, label: &str) -> Div {
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
                    .child(label.to_string())
            )
    }

    fn render_separator(&self) -> Div {
        let theme = &self.theme;

        div()
            .w_full()
            .h(px(1.0))
            .my(px(4.0))
            .bg(theme.border)
    }
}