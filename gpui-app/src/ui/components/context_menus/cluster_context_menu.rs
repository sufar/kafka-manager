//! Cluster Context Menu
//!
//! Context menu for cluster nodes in the tree navigator.
//! Integrates with GlobalState Entity for real actions.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::state::GlobalState;
use crate::i18n::Translations;
use crate::router::ViewType;
use std::sync::Arc;

/// Actions available from cluster context menu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClusterAction {
    TestConnection,
    RefreshStatus,
    Disconnect,
    Reconnect,
    ViewBrokers,
    ViewTopics,
    RefreshTopics,
    ViewConsumerGroups,
    CreateTopic,
    EditCluster,
    DeleteCluster,
}

impl ClusterAction {
    pub fn label(&self, t: &Translations) -> String {
        match self {
            ClusterAction::TestConnection => t.clusters.testing_connection.clone(),
            ClusterAction::RefreshStatus => "Refresh Status".to_string(),
            ClusterAction::Disconnect => "Disconnect".to_string(),
            ClusterAction::Reconnect => "Reconnect".to_string(),
            ClusterAction::ViewBrokers => "View Brokers".to_string(),
            ClusterAction::ViewTopics => t.clusters.title.clone(),
            ClusterAction::RefreshTopics => t.common.refresh.clone(),
            ClusterAction::ViewConsumerGroups => t.consumer_groups.title.clone(),
            ClusterAction::CreateTopic => t.topics.title.clone(),
            ClusterAction::EditCluster => t.common.edit.clone(),
            ClusterAction::DeleteCluster => t.common.delete.clone(),
        }
    }
}

/// Cluster context menu with GlobalState integration
pub struct ClusterContextMenuWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    cluster_name: String,
    position: Point<Pixels>,
    visible: bool,
}

impl ClusterContextMenuWithState {
    pub fn new(
        state: Entity<GlobalState>,
        translations: Arc<Translations>,
        cluster_name: String,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            state,
            translations,
            cluster_name,
            position,
            visible: true,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn handle_action(&mut self, action: ClusterAction, cx: &mut Context<Self>) {
        let cluster_name = self.cluster_name.clone();
        match action {
            ClusterAction::TestConnection => {
                self.state.update(cx, |state, cx| {
                    state.check_cluster_health(&cluster_name, cx);
                    cx.notify();
                });
            }
            ClusterAction::RefreshStatus => {
                self.state.update(cx, |state, cx| {
                    state.check_cluster_health(&cluster_name, cx);
                    cx.notify();
                });
            }
            ClusterAction::ViewTopics => {
                self.state.update(cx, |state, cx| {
                    state.navigate_to_topics(&cluster_name);
                    cx.notify();
                });
            }
            ClusterAction::RefreshTopics => {
                self.state.update(cx, |state, cx| {
                    state.load_cluster_topics(&cluster_name, cx);
                    cx.notify();
                });
            }
            ClusterAction::ViewConsumerGroups => {
                self.state.update(cx, |state, cx| {
                    state.load_cluster_consumer_groups(&cluster_name, cx);
                    state.navigate(ViewType::ConsumerGroups);
                    cx.notify();
                });
            }
            _ => {}
        }
        self.visible = false;
    }
}

impl Render for ClusterContextMenuWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("cluster-context-menu-hidden");
        }

        let theme = self.state.read(cx).theme.clone();
        let t = &self.translations;
        let cluster_name = self.cluster_name.clone();
        let health = self.state.read(cx).cluster_health.get(&cluster_name);
        let is_connected = health.map(|h| h.healthy).unwrap_or(false);

        div()
            .id("cluster-context-menu")
            .absolute()
            .top(self.position.y)
            .left(self.position.x)
            .w(px(200.0))
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
                                    .child(cluster_name.clone())
                            )
                    )
                    // Connection actions - inline with click handlers
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("action-test-connection")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::TestConnection.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::TestConnection, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-refresh-status")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::RefreshStatus.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::RefreshStatus, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-disconnect")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(!is_connected, |d| d.opacity(0.5))
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::Disconnect.label(t)))
                                    .when(is_connected, |this| {
                                        this.on_click(cx.listener(|this, _, _, cx| {
                                            this.handle_action(ClusterAction::Disconnect, cx);
                                        }))
                                    })
                            )
                            .child(
                                div()
                                    .id("action-reconnect")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(is_connected, |d| d.opacity(0.5))
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::Reconnect.label(t)))
                                    .when(!is_connected, |this| {
                                        this.on_click(cx.listener(|this, _, _, cx| {
                                            this.handle_action(ClusterAction::Reconnect, cx);
                                        }))
                                    })
                            )
                    )
                    // Separator
                    .child(div().w_full().h(px(1.0)).my(px(4.0)).bg(theme.border))
                    // Navigation actions
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("action-view-brokers")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::ViewBrokers.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::ViewBrokers, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-view-topics")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::ViewTopics.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::ViewTopics, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-refresh-topics")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::RefreshTopics.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::RefreshTopics, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-view-consumer-groups")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::ViewConsumerGroups.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::ViewConsumerGroups, cx);
                                    }))
                            )
                    )
                    // Separator
                    .child(div().w_full().h(px(1.0)).my(px(4.0)).bg(theme.border))
                    // Management actions
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("action-create-topic")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::CreateTopic.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::CreateTopic, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-edit-cluster")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(div().text_color(theme.text).text_sm().child(ClusterAction::EditCluster.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::EditCluster, cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("action-delete-cluster")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.error.opacity(0.1)))
                                    .child(div().text_color(theme.error).text_sm().child(ClusterAction::DeleteCluster.label(t)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_action(ClusterAction::DeleteCluster, cx);
                                    }))
                            )
                    )
            )
    }
}

/// Legacy cluster context menu (static version)
#[derive(Clone)]
pub struct ClusterContextMenu {
    theme: Theme,
    cluster_name: String,
    position: Point<Pixels>,
    is_testing: bool,
    is_refreshing: bool,
    is_disconnecting: bool,
    is_reconnecting: bool,
}

impl ClusterContextMenu {
    pub fn new(
        theme: Theme,
        cluster_name: String,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            theme,
            cluster_name,
            position,
            is_testing: false,
            is_refreshing: false,
            is_disconnecting: false,
            is_reconnecting: false,
        }
    }

    pub fn set_testing(&mut self, testing: bool) {
        self.is_testing = testing;
    }

    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.is_refreshing = refreshing;
    }

    pub fn set_disconnecting(&mut self, disconnecting: bool) {
        self.is_disconnecting = disconnecting;
    }

    pub fn set_reconnecting(&mut self, reconnecting: bool) {
        self.is_reconnecting = reconnecting;
    }
}

impl IntoElement for ClusterContextMenu {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .absolute()
            .top(self.position.y)
            .left(self.position.x)
            .w(px(200.0))
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
                                    .child(self.cluster_name.clone())
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("测试连接", self.is_testing))
                            .child(self.render_item("刷新状态", self.is_refreshing))
                            .child(self.render_item("断开连接", self.is_disconnecting))
                            .child(self.render_item("重新连接", self.is_reconnecting))
                    )
                    .child(self.render_separator())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("查看 Brokers", false))
                            .child(self.render_item("查看 Topics", false))
                            .child(self.render_item("刷新 Topics", self.is_refreshing))
                            .child(self.render_item("查看 Consumer Groups", false))
                    )
                    .child(self.render_separator())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("创建 Topic", false))
                            .child(self.render_item("编辑集群", false))
                            .child(self.render_danger_item("删除集群"))
                    )
            )
    }
}

impl ClusterContextMenu {
    fn render_item(&self, label: &str, is_disabled: bool) -> Div {
        let theme = &self.theme;
        let opacity = if is_disabled { 0.5 } else { 1.0 };

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .when(is_disabled, |d| d.opacity(opacity))
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