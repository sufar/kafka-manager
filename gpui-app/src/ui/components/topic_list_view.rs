//! Topic List View Component
//!
//! Detailed view showing topic configuration, partitions, and statistics.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::api::PartitionInfo;

/// Topic configuration entry
#[derive(Debug, Clone)]
pub struct TopicConfigEntry {
    /// Config key name
    pub key: String,
    /// Config value
    pub value: String,
    /// Is default config
    pub is_default: bool,
    /// Description
    pub description: Option<String>,
}

/// Topic statistics
#[derive(Debug, Clone, Default)]
pub struct TopicStats {
    /// Total messages
    pub total_messages: i64,
    /// Messages per partition
    pub messages_per_partition: Vec<i64>,
    /// Bytes per partition
    pub bytes_per_partition: Vec<i64>,
    /// Last produce timestamp
    pub last_produce: Option<i64>,
    /// Consumer group count
    pub consumer_groups: i32,
}

/// Topic List View - Detailed topic information panel
pub struct TopicListView {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Topic name
    topic_name: String,
    /// Cluster name
    cluster_name: String,
    /// Number of partitions
    partitions: i32,
    /// Replication factor
    replication_factor: i32,
    /// Partition details
    partition_details: Vec<PartitionInfo>,
    /// Topic configuration
    config: Vec<TopicConfigEntry>,
    /// Topic statistics
    stats: TopicStats,
    /// Selected tab
    selected_tab: TopicTab,
    /// Expanded partition
    expanded_partition: Option<i32>,
}

/// Tab selection for topic view
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TopicTab {
    #[default]
    Overview,
    Partitions,
    Config,
    Stats,
}

impl TopicTab {
    /// Get display label
    pub fn label(&self, translations: &Translations) -> String {
        match self {
            TopicTab::Overview => "概览".to_string(),
            TopicTab::Partitions => translations.topics.partitions.clone(),
            TopicTab::Config => "配置".to_string(),
            TopicTab::Stats => "统计".to_string(),
        }
    }
}

impl TopicListView {
    /// Create new topic list view
    pub fn new(theme: Theme, translations: Arc<Translations>, topic_name: String, cluster_name: String) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            topic_name,
            cluster_name,
            partitions: 3,
            replication_factor: 1,
            partition_details: vec![
                PartitionInfo { partition: 0, leader: 1, replicas: vec![1, 2], isr: vec![1, 2] },
                PartitionInfo { partition: 1, leader: 2, replicas: vec![2, 1], isr: vec![2] },
                PartitionInfo { partition: 2, leader: 1, replicas: vec![1, 2], isr: vec![1, 2] },
            ],
            config: vec![
                TopicConfigEntry { key: "retention.ms".to_string(), value: "604800000".to_string(), is_default: true, description: Some("消息保留时间".to_string()) },
                TopicConfigEntry { key: "segment.bytes".to_string(), value: "1073741824".to_string(), is_default: true, description: Some("段文件大小".to_string()) },
                TopicConfigEntry { key: "compression.type".to_string(), value: "producer".to_string(), is_default: true, description: Some("压缩类型".to_string()) },
                TopicConfigEntry { key: "max.message.bytes".to_string(), value: "1000012".to_string(), is_default: true, description: None },
            ],
            stats: TopicStats {
                total_messages: 1234567,
                messages_per_partition: vec![401234, 400000, 433333],
                bytes_per_partition: vec![1024 * 1024 * 500, 1024 * 1024 * 480, 1024 * 1024 * 520],
                last_produce: Some(1716432600000),
                consumer_groups: 5,
            },
            selected_tab: TopicTab::default(),
            expanded_partition: None,
        }
    }

    /// Set topic details
    pub fn set_topic(&mut self, name: String, partitions: i32, replication: i32) {
        self.topic_name = name;
        self.partitions = partitions;
        self.replication_factor = replication;
    }

    /// Set partition details
    pub fn set_partitions(&mut self, details: Vec<PartitionInfo>) {
        self.partition_details = details;
    }

    /// Set configuration
    pub fn set_config(&mut self, config: Vec<TopicConfigEntry>) {
        self.config = config;
    }

    /// Set statistics
    pub fn set_stats(&mut self, stats: TopicStats) {
        self.stats = stats;
    }

    /// Set selected tab
    pub fn set_tab(&mut self, tab: TopicTab) {
        self.selected_tab = tab;
    }

    /// Toggle partition expansion
    pub fn toggle_partition(&mut self, partition: i32) {
        if self.expanded_partition == Some(partition) {
            self.expanded_partition = None;
        } else {
            self.expanded_partition = Some(partition);
        }
    }

    /// Render tab selector
    fn render_tabs(&self) -> Div {
        let theme = &self.theme;
        let tabs = [TopicTab::Overview, TopicTab::Partitions, TopicTab::Config, TopicTab::Stats];

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .p(px(4.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .children(tabs.iter().map(|tab| {
                let is_selected = self.selected_tab == *tab;
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .bg(if is_selected { theme.primary } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(if is_selected {
                                Hsla::from(gpui::rgb(0xffffff))
                            } else {
                                theme.text_secondary
                            })
                            .text_xs()
                            .child(tab.label(&self.translations))
                    )
            }))
    }

    /// Render overview tab
    fn render_overview(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(12.0))
            .child(
                // Topic header
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.topic_name.clone())
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(self.cluster_name.clone())
                            )
                    )
            )
            .child(
                // Quick stats row
                div()
                    .flex()
                    .items_center()
                    .gap(px(24.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(t.topics.partitions.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.partitions.to_string())
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
                                    .child(t.topics.replication_factor.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.replication_factor.to_string())
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
                                    .child("总消息数")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format_number(self.stats.total_messages))
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
                                    .child("消费者组")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.stats.consumer_groups.to_string())
                            )
                    )
            )
            .child(
                // ISR summary
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("ISR 状态")
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .children(self.partition_details.iter().map(|p| {
                                let isr_ok = p.isr.len() >= self.replication_factor as usize;
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(6.0))
                                    .py(px(3.0))
                                    .rounded(px(4.0))
                                    .bg(if isr_ok { theme.success.opacity(0.1) } else { theme.warning.opacity(0.1) })
                                    .child(
                                        div()
                                            .text_color(if isr_ok { theme.success } else { theme.warning })
                                            .text_xs()
                                            .child(format!("P{}: {}", p.partition, p.isr.len()))
                                    )
                            }))
                    )
            )
    }

    /// Render partitions tab
    fn render_partitions(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .child(
                // Partitions table header
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(theme.surface_raised)
                    .rounded(px(4.0))
                    .child(
                        div()
                            .w(px(50.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("分区")
                    )
                    .child(
                        div()
                            .w(px(60.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("Leader")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("Replicas")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("ISR")
                    )
            )
            .children(self.partition_details.iter().enumerate().map(|(idx, p)| {
                let is_expanded = self.expanded_partition == Some(p.partition);
                let is_odd = idx % 2 == 1;

                div()
                    .flex()
                    .flex_col()
                    .child(
                        // Partition row
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .px(px(8.0))
                            .py(px(6.0))
                            .bg(if is_odd { theme.surface } else { gpui::transparent_black() })
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .child(
                                div()
                                    .w(px(50.0))
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("P{}", p.partition))
                            )
                            .child(
                                div()
                                    .w(px(60.0))
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(p.leader.to_string())
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(format!("{:?}", p.replicas))
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(if p.isr.len() >= self.replication_factor as usize {
                                        theme.success
                                    } else {
                                        theme.warning
                                    })
                                    .text_xs()
                                    .child(format!("{:?}", p.isr))
                            )
                            .child(
                                // Expand indicator
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(4.0))
                                    .bg(theme.surface_raised)
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(if is_expanded { "−" } else { "+" })
                                    )
                            )
                    )
                    .when(is_expanded, |this| {
                        this.child(
                            // Expanded details
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .ml(px(20.0))
                                .p(px(8.0))
                                .rounded(px(4.0))
                                .bg(theme.surface)
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child(format!("消息数: {}", format_number(self.stats.messages_per_partition.get(idx as usize).copied().unwrap_or(0))))
                                )
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child(format!("大小: {}", format_bytes(self.stats.bytes_per_partition.get(idx as usize).copied().unwrap_or(0))))
                                )
                        )
                    })
            }))
    }

    /// Render config tab
    fn render_config(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .child(
                // Config table header
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(theme.surface_raised)
                    .rounded(px(4.0))
                    .child(
                        div()
                            .w(px(200.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("配置项")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("值")
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("来源")
                    )
            )
            .children(self.config.iter().enumerate().map(|(idx, entry)| {
                let is_odd = idx % 2 == 1;

                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(if is_odd { theme.surface } else { gpui::transparent_black() })
                    .rounded(px(4.0))
                    .child(
                        div()
                            .w(px(200.0))
                            .text_color(theme.text)
                            .text_xs()
                            .child(entry.key.clone())
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .truncate()
                            .child(entry.value.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(if entry.is_default { theme.surface } else { theme.primary.opacity(0.1) })
                            .child(
                                div()
                                    .text_color(if entry.is_default { theme.text_muted } else { theme.primary })
                                    .text_xs()
                                    .child(if entry.is_default { "默认" } else { "自定义" })
                            )
                    )
                    .when_some(entry.description.clone(), |this, desc| {
                        this.child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(desc)
                        )
                    })
            }))
    }

    /// Render stats tab
    fn render_stats(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(12.0))
            .child(
                // Total messages chart placeholder
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("消息分布")
                    )
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .gap(px(4.0))
                            .h(px(60.0))
                            .children(self.stats.messages_per_partition.iter().enumerate().map(|(idx, count)| {
                                let max = self.stats.messages_per_partition.iter().max().copied().unwrap_or(1);
                                let height_pct = *count as f32 / max as f32;
                                div()
                                    .flex()
                                    .flex_col()
                                    .justify_end()
                                    .w(px(40.0))
                                    .child(
                                        div()
                                            .h(px(height_pct * 50.0))
                                            .rounded(px(2.0))
                                            .bg(theme.primary)
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(format!("P{}", idx))
                                    )
                            }))
                    )
            )
            .child(
                // Stats details
                div()
                    .flex()
                    .items_center()
                    .gap(px(24.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("最后生产时间")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(self.stats.last_produce.map(|ts| format_timestamp(ts)).unwrap_or_else(|| "未知".to_string()))
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
                                    .child("总大小")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(format_bytes(self.stats.bytes_per_partition.iter().sum()))
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

/// Format bytes to human readable
fn format_bytes(bytes: i64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{} GB", bytes / (1024 * 1024 * 1024))
    } else if bytes >= 1024 * 1024 {
        format!("{} MB", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}

/// Format timestamp
fn format_timestamp(ts_ms: i64) -> String {
    use chrono::{Utc, TimeZone};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts_ms.to_string())
}

impl IntoElement for TopicListView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .h_full()
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .p(px(8.0))
            .child(self.render_tabs())
            .child(
                div()
                    .flex_1()
                    .overflow_y()
                    .child(match self.selected_tab {
                        TopicTab::Overview => self.render_overview(),
                        TopicTab::Partitions => self.render_partitions(),
                        TopicTab::Config => self.render_config(),
                        TopicTab::Stats => self.render_stats(),
                    })
            )
    }
}

impl Clone for TopicListView {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            topic_name: self.topic_name.clone(),
            cluster_name: self.cluster_name.clone(),
            partitions: self.partitions,
            replication_factor: self.replication_factor,
            partition_details: self.partition_details.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
            selected_tab: self.selected_tab,
            expanded_partition: self.expanded_partition,
        }
    }
}