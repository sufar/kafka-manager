//! Navigator Status Bar Component
//!
//! Status bar for navigation sidebar showing connection status and message statistics.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::state::ConnectionStatusType;

/// Status item for display
#[derive(Debug, Clone)]
pub struct StatusItem {
    /// Item label
    pub label: String,
    /// Item value
    pub value: String,
    /// Status type (for coloring)
    pub status: StatusItemType,
}

/// Status item type for coloring
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StatusItemType {
    #[default]
    Neutral,
    Success,
    Warning,
    Error,
    Info,
}

impl StatusItemType {
    /// Get color for this status type
    pub fn color(&self, theme: &Theme) -> Hsla {
        match self {
            StatusItemType::Neutral => theme.text_muted,
            StatusItemType::Success => theme.success,
            StatusItemType::Warning => theme.warning,
            StatusItemType::Error => theme.error,
            StatusItemType::Info => theme.primary,
        }
    }
}

/// Connection summary
#[derive(Debug, Clone)]
pub struct ConnectionSummary {
    /// Cluster name
    pub cluster: String,
    /// Connection status
    pub status: ConnectionStatusType,
    /// Broker count
    pub brokers: i32,
    /// Topic count
    pub topics: i32,
}

/// Message statistics
#[derive(Debug, Clone, Default)]
pub struct MessageStats {
    /// Total messages received today
    pub messages_today: i64,
    /// Total bytes received today
    pub bytes_today: i64,
    /// Active streams
    pub active_streams: i32,
    /// Messages per second rate
    pub rate_per_sec: f32,
}

/// Navigator Status Bar - Shows connection and message status
pub struct NavigatorStatusBar {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Connection summaries
    connections: Vec<ConnectionSummary>,
    /// Message statistics
    message_stats: MessageStats,
    /// Is collapsed
    is_collapsed: bool,
}

impl NavigatorStatusBar {
    /// Create new status bar
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            connections: vec![
                ConnectionSummary {
                    cluster: "Production".to_string(),
                    status: ConnectionStatusType::Connected,
                    brokers: 3,
                    topics: 12,
                },
                ConnectionSummary {
                    cluster: "Development".to_string(),
                    status: ConnectionStatusType::Disconnected,
                    brokers: 1,
                    topics: 2,
                },
            ],
            message_stats: MessageStats {
                messages_today: 1234567,
                bytes_today: 1024 * 1024 * 500,
                active_streams: 2,
                rate_per_sec: 125.5,
            },
            is_collapsed: false,
        }
    }

    /// Set connection summaries
    pub fn set_connections(&mut self, connections: Vec<ConnectionSummary>) {
        self.connections = connections;
    }

    /// Set message statistics
    pub fn set_message_stats(&mut self, stats: MessageStats) {
        self.message_stats = stats;
    }

    /// Toggle collapsed state
    pub fn toggle_collapsed(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    /// Get connected count
    fn connected_count(&self) -> i32 {
        self.connections.iter()
            .filter(|c| c.status == ConnectionStatusType::Connected)
            .count() as i32
    }

    /// Get total connections
    fn total_connections(&self) -> i32 {
        self.connections.len() as i32
    }

    /// Render connection status indicator
    fn render_connection_indicator(&self, status: ConnectionStatusType) -> Div {
        let theme = &self.theme;
        let color = match status {
            ConnectionStatusType::Connected => theme.success,
            ConnectionStatusType::Disconnected => theme.text_muted,
            ConnectionStatusType::Error => theme.error,
            ConnectionStatusType::Unknown => theme.warning,
        };

        div()
            .w(px(10.0))
            .h(px(10.0))
            .rounded(px(5.0))
            .bg(color)
    }

    /// Render collapsed status bar
    fn render_collapsed(&self) -> Div {
        let theme = &self.theme;
        let connected = self.connected_count();
        let total = self.total_connections();

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .w(px(40.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(theme.surface)
            .child(
                // Connection count
                div()
                    .text_color(if connected == total { theme.success } else { theme.warning })
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("{}/{}", connected, total))
            )
    }

    /// Render expanded status bar
    fn render_expanded(&self) -> Div {
        let theme = &self.theme;
        let connected = self.connected_count();
        let total = self.total_connections();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .p(px(8.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Connections summary row
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("连接:")
                    )
                    .child(
                        div()
                            .text_color(if connected == total { theme.success } else { theme.warning })
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("{}/{}", connected, total))
                    )
            )
            .child(
                // Connection details
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .children(self.connections.iter().map(|conn| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(self.render_connection_indicator(conn.status))
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(conn.cluster.clone())
                            )
                            .child(
                                div()
                                    .px(px(4.0))
                                    .py(px(2.0))
                                    .rounded(px(3.0))
                                    .bg(theme.surface_raised)
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(format!("{}T", conn.topics))
                                    )
                            )
                    }))
            )
            .child(
                // Message stats row
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .pt(px(4.0))
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("今日消息")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(format_number(self.message_stats.messages_today))
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("活跃流")
                            )
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child(format!("{}", self.message_stats.active_streams))
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("速率")
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(format!("{}/s", self.message_stats.rate_per_sec as i32))
                            )
                    )
            )
    }
}

/// Format number with thousands separator
fn format_number(n: i64) -> String {
    if n >= 1000000 {
        format!("{}M", n / 1000000)
    } else if n >= 1000 {
        format!("{}K", n / 1000)
    } else {
        n.to_string()
    }
}

impl IntoElement for NavigatorStatusBar {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if self.is_collapsed {
            self.render_collapsed()
        } else {
            self.render_expanded()
        }
    }
}

impl Clone for NavigatorStatusBar {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            connections: self.connections.clone(),
            message_stats: self.message_stats.clone(),
            is_collapsed: self.is_collapsed,
        }
    }
}