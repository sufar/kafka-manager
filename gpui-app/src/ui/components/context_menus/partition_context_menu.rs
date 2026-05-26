//! Partition Context Menu
//!
//! Context menu for partition nodes in the tree navigator.
//! Integrates with GlobalState Entity for real actions.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::state::GlobalState;
use std::sync::Arc;
use crate::i18n::Translations;

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

/// Partition context menu with GlobalState integration
pub struct PartitionContextMenuWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    topic_name: String,
    cluster_name: String,
    partition_id: i32,
    position: Point<Pixels>,
    visible: bool,
}

impl PartitionContextMenuWithState {
    pub fn new(
        state: Entity<GlobalState>,
        translations: Arc<Translations>,
        topic_name: String,
        cluster_name: String,
        partition_id: i32,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            state,
            translations,
            topic_name,
            cluster_name,
            partition_id,
            position,
            visible: true,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn handle_action(&mut self, action: PartitionAction, cx: &mut Context<Self>) {
        let cluster_name = self.cluster_name.clone();
        let topic_name = self.topic_name.clone();
        let partition_id = self.partition_id;

        match action {
            PartitionAction::ViewMessages => {
                self.state.update(cx, |state, cx| {
                    state.select_topic(cluster_name.clone(), topic_name.clone());
                    // Set partition filter
                    state.set_query_partition(Some(partition_id));
                    state.navigate_to_messages(&cluster_name, &topic_name);
                    cx.notify();
                });
            }
            PartitionAction::SendMessage => {
                println!("Send message to partition {} of topic: {} in cluster: {}", partition_id, topic_name, cluster_name);
            }
        }
        self.visible = false;
    }
}

impl Render for PartitionContextMenuWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("partition-context-menu-hidden");
        }

        let state = self.state.read(cx);
        let theme = &state.theme;
        let t = &self.translations;
        let title = format!("{} #{}", self.topic_name, self.partition_id);

        div()
            .id("partition-context-menu-state")
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
                                    .child(title)
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("partition-action-view-messages")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(t.messages.title.clone()))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(PartitionAction::ViewMessages, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("partition-action-send-message")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child("Send Message"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(PartitionAction::SendMessage, cx);
                                    }))
                            )
                    )
            )
    }
}