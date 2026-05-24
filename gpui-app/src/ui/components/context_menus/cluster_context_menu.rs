//! Cluster Context Menu
//!
//! Context menu for cluster nodes in the tree navigator.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

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

/// Cluster context menu
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
    /// Create new cluster context menu
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

    /// Set testing state
    pub fn set_testing(&mut self, testing: bool) {
        self.is_testing = testing;
    }

    /// Set refreshing state
    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.is_refreshing = refreshing;
    }

    /// Set disconnecting state
    pub fn set_disconnecting(&mut self, disconnecting: bool) {
        self.is_disconnecting = disconnecting;
    }

    /// Set reconnecting state
    pub fn set_reconnecting(&mut self, reconnecting: bool) {
        self.is_reconnecting = reconnecting;
    }

    /// Render menu item
    fn render_item(&self, label: String, _action: ClusterAction, is_disabled: bool) -> Div {
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
                                    .child(self.cluster_name.clone())
                            )
                    )
                    .child(
                        // Connection actions
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("测试连接".to_string(), ClusterAction::TestConnection, self.is_testing))
                            .child(self.render_item("刷新状态".to_string(), ClusterAction::RefreshStatus, self.is_refreshing))
                            .child(self.render_item("断开连接".to_string(), ClusterAction::Disconnect, self.is_disconnecting))
                            .child(self.render_item("重新连接".to_string(), ClusterAction::Reconnect, self.is_reconnecting))
                    )
                    .child(self.render_separator())
                    .child(
                        // Navigation actions
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("查看 Brokers".to_string(), ClusterAction::ViewBrokers, false))
                            .child(self.render_item("查看 Topics".to_string(), ClusterAction::ViewTopics, false))
                            .child(self.render_item("刷新 Topics".to_string(), ClusterAction::RefreshTopics, self.is_refreshing))
                            .child(self.render_item("查看 Consumer Groups".to_string(), ClusterAction::ViewConsumerGroups, false))
                    )
                    .child(self.render_separator())
                    .child(
                        // Management actions
                        div()
                            .flex()
                            .flex_col()
                            .py(px(4.0))
                            .child(self.render_item("创建 Topic".to_string(), ClusterAction::CreateTopic, false))
                            .child(self.render_item("编辑集群".to_string(), ClusterAction::EditCluster, false))
                            .child(self.render_danger_item("删除集群".to_string()))
                    )
            )
    }
}