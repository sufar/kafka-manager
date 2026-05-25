//! Topic Consumer Groups View
//!
//! View for displaying consumer groups associated with a specific topic.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Consumer group member info
#[derive(Debug, Clone)]
pub struct ConsumerMember {
    /// Member ID
    pub member_id: String,
    /// Client ID
    pub client_id: String,
    /// Host address
    pub host: String,
    /// Assigned partitions
    pub partitions: Vec<i32>,
}

/// Partition assignment info
#[derive(Debug, Clone)]
pub struct PartitionAssignment {
    /// Partition number
    pub partition: i32,
    /// Assigned consumer member ID
    pub member_id: Option<String>,
    /// Current offset
    pub current_offset: i64,
    /// End offset (latest)
    pub end_offset: i64,
    /// Lag (end_offset - current_offset)
    pub lag: i64,
}

/// Consumer group info for a topic
#[derive(Debug, Clone)]
pub struct TopicConsumerGroup {
    /// Group ID
    pub group_id: String,
    /// Group state
    pub state: ConsumerGroupState,
    /// Number of members
    pub member_count: i32,
    /// Total lag for this topic
    pub total_lag: i64,
    /// Members list
    pub members: Vec<ConsumerMember>,
    /// Partition assignments
    pub assignments: Vec<PartitionAssignment>,
}

/// Consumer group state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumerGroupState {
    Stable,
    PreparingRebalance,
    CompletingRebalance,
    Empty,
    Dead,
}

impl ConsumerGroupState {
    /// Get display color
    pub fn color(&self, theme: &Theme) -> Hsla {
        match self {
            ConsumerGroupState::Stable => theme.success,
            ConsumerGroupState::PreparingRebalance => theme.warning,
            ConsumerGroupState::CompletingRebalance => theme.info,
            ConsumerGroupState::Empty => theme.text_muted,
            ConsumerGroupState::Dead => theme.error,
        }
    }

    /// Get display label
    pub fn label(&self) -> String {
        match self {
            ConsumerGroupState::Stable => "稳定".to_string(),
            ConsumerGroupState::PreparingRebalance => "重平衡".to_string(),
            ConsumerGroupState::CompletingRebalance => "完成重平衡".to_string(),
            ConsumerGroupState::Empty => "空".to_string(),
            ConsumerGroupState::Dead => "已死".to_string(),
        }
    }
}

/// Topic Consumer Groups View
pub struct TopicConsumerGroupsView {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Topic name
    topic_name: String,
    /// Cluster name
    cluster_name: String,
    /// Consumer groups
    consumer_groups: Vec<TopicConsumerGroup>,
    /// Selected group
    selected_group: Option<usize>,
    /// Selected tab (Groups / Members / Partitions)
    selected_tab: ConsumerGroupTab,
    /// Search query
    search_query: String,
}

/// Tab selection for consumer groups view
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConsumerGroupTab {
    #[default]
    Groups,
    Members,
    Partitions,
}

impl ConsumerGroupTab {
    /// Get display label
    pub fn label(&self) -> String {
        match self {
            ConsumerGroupTab::Groups => "消费者组".to_string(),
            ConsumerGroupTab::Members => "成员".to_string(),
            ConsumerGroupTab::Partitions => "分区分配".to_string(),
        }
    }
}

impl TopicConsumerGroupsView {
    /// Create new view
    pub fn new(theme: Theme, translations: Arc<Translations>, topic_name: String, cluster_name: String) -> Self {
        let consumer_groups = vec![
            TopicConsumerGroup {
                group_id: "order-processors".to_string(),
                state: ConsumerGroupState::Stable,
                member_count: 3,
                total_lag: 1250,
                members: vec![
                    ConsumerMember {
                        member_id: "consumer-1".to_string(),
                        client_id: "order-app-1".to_string(),
                        host: "192.168.1.10".to_string(),
                        partitions: vec![0, 1, 2],
                    },
                    ConsumerMember {
                        member_id: "consumer-2".to_string(),
                        client_id: "order-app-2".to_string(),
                        host: "192.168.1.11".to_string(),
                        partitions: vec![3, 4, 5],
                    },
                    ConsumerMember {
                        member_id: "consumer-3".to_string(),
                        client_id: "order-app-3".to_string(),
                        host: "192.168.1.12".to_string(),
                        partitions: vec![6, 7, 8],
                    },
                ],
                assignments: vec![
                    PartitionAssignment { partition: 0, member_id: Some("consumer-1".to_string()), current_offset: 1000000, end_offset: 1000125, lag: 125 },
                    PartitionAssignment { partition: 1, member_id: Some("consumer-1".to_string()), current_offset: 999000, end_offset: 999500, lag: 500 },
                    PartitionAssignment { partition: 2, member_id: Some("consumer-1".to_string()), current_offset: 998000, end_offset: 998625, lag: 625 },
                ],
            },
            TopicConsumerGroup {
                group_id: "order-analytics".to_string(),
                state: ConsumerGroupState::Stable,
                member_count: 1,
                total_lag: 0,
                members: vec![
                    ConsumerMember {
                        member_id: "analytics-1".to_string(),
                        client_id: "analytics-app".to_string(),
                        host: "192.168.1.20".to_string(),
                        partitions: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                    },
                ],
                assignments: vec![
                    PartitionAssignment { partition: 0, member_id: Some("analytics-1".to_string()), current_offset: 1000125, end_offset: 1000125, lag: 0 },
                    PartitionAssignment { partition: 1, member_id: Some("analytics-1".to_string()), current_offset: 999500, end_offset: 999500, lag: 0 },
                ],
            },
            TopicConsumerGroup {
                group_id: "test-consumer".to_string(),
                state: ConsumerGroupState::Empty,
                member_count: 0,
                total_lag: 5000,
                members: vec![],
                assignments: vec![
                    PartitionAssignment { partition: 0, member_id: None, current_offset: 950000, end_offset: 955000, lag: 5000 },
                ],
            },
        ];

        Self {
            theme: theme.clone(),
            translations,
            topic_name,
            cluster_name,
            consumer_groups,
            selected_group: None,
            selected_tab: ConsumerGroupTab::default(),
            search_query: String::new(),
        }
    }

    /// Set consumer groups
    pub fn set_consumer_groups(&mut self, groups: Vec<TopicConsumerGroup>) {
        self.consumer_groups = groups;
    }

    /// Set selected group
    pub fn set_selected_group(&mut self, index: usize) {
        self.selected_group = Some(index);
    }

    /// Set selected tab
    pub fn set_tab(&mut self, tab: ConsumerGroupTab) {
        self.selected_tab = tab;
    }

    /// Set search query
    pub fn set_search(&mut self, query: String) {
        self.search_query = query;
    }

    /// Get selected group
    pub fn selected_group(&self) -> Option<&TopicConsumerGroup> {
        self.selected_group.map(|idx| &self.consumer_groups[idx])
    }

    /// Render tabs
    fn render_tabs(&self) -> Div {
        let theme = &self.theme;
        let tabs = [ConsumerGroupTab::Groups, ConsumerGroupTab::Members, ConsumerGroupTab::Partitions];

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
                            .child(tab.label())
                    )
            }))
    }

    /// Render groups tab
    fn render_groups(&self) -> Div {
        let theme = &self.theme;
        let filtered = self.filtered_groups();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .child(
                // Groups table header
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
                            .child("状态")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("消费者组")
                    )
                    .child(
                        div()
                            .w(px(60.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("成员")
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("Lag")
                    )
            )
            .children(filtered.iter().enumerate().map(|(idx, group)| {
                let global_idx = self.consumer_groups.iter().position(|g| g.group_id == group.group_id).unwrap_or(0);
                let is_selected = self.selected_group == Some(global_idx);
                let is_odd = idx % 2 == 1;

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
                        // Status indicator
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(5.0))
                            .bg(group.state.color(theme))
                    )
                    .child(
                        // Group ID
                        div()
                            .flex_1()
                            .text_color(theme.text)
                            .text_sm()
                            .child(group.group_id.clone())
                    )
                    .child(
                        // Member count
                        div()
                            .w(px(60.0))
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(format!("{}", group.member_count))
                    )
                    .child(
                        // Lag
                        div()
                            .w(px(80.0))
                            .text_color(if group.total_lag > 0 { theme.warning } else { theme.success })
                            .text_xs()
                            .child(format_lag(group.total_lag))
                    )
            }))
    }

    /// Render members tab (for selected group)
    fn render_members(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .when_some(self.selected_group(), |this, group| {
                this.child(
                    // Group header
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .pb(px(6.0))
                        .border_b(px(1.0))
                        .border_color(theme.border)
                        .child(
                            div()
                                .text_color(theme.text)
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(group.group_id.clone())
                        )
                        .child(
                            div()
                                .px(px(4.0))
                                .py(px(2.0))
                                .rounded(px(3.0))
                                .bg(group.state.color(theme).opacity(0.2))
                                .child(
                                    div()
                                        .text_color(group.state.color(theme))
                                        .text_xs()
                                        .child(group.state.label())
                                )
                        )
                )
                .child(
                    // Members table header
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
                                .flex_1()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("成员ID")
                        )
                        .child(
                            div()
                                .w(px(120.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("客户端ID")
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("主机")
                        )
                        .child(
                            div()
                                .w(px(80.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("分区")
                        )
                )
                .children(group.members.iter().enumerate().map(|(idx, member)| {
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
                                .flex_1()
                                .text_color(theme.text)
                                .text_xs()
                                .child(member.member_id.clone())
                        )
                        .child(
                            div()
                                .w(px(120.0))
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .truncate()
                                .child(member.client_id.clone())
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(member.host.clone())
                        )
                        .child(
                            div()
                                .w(px(80.0))
                                .text_color(theme.primary)
                                .text_xs()
                                .child(format!("{:?}", member.partitions))
                        )
                }))
            })
            .when(self.selected_group().is_none(), |this| {
                this.child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child("请先选择一个消费者组")
                )
            })
    }

    /// Render partitions tab (for selected group)
    fn render_partitions(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .when_some(self.selected_group(), |this, group| {
                this.child(
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
                                .w(px(60.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("分区")
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("消费者")
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("当前偏移")
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("末尾偏移")
                        )
                        .child(
                            div()
                                .w(px(80.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("Lag")
                        )
                )
                .children(group.assignments.iter().enumerate().map(|(idx, assignment)| {
                    let is_odd = idx % 2 == 1;
                    let lag_color = if assignment.lag > 1000 { theme.error }
                        else if assignment.lag > 0 { theme.warning }
                        else { theme.success };

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
                                .w(px(60.0))
                                .text_color(theme.primary)
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("P{}", assignment.partition))
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text)
                                .text_xs()
                                .child(assignment.member_id.clone().unwrap_or_else(|| "无".to_string()))
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .child(format!("{}", assignment.current_offset))
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .child(format!("{}", assignment.end_offset))
                        )
                        .child(
                            div()
                                .w(px(80.0))
                                .text_color(lag_color)
                                .text_xs()
                                .child(format_lag(assignment.lag))
                        )
                }))
            })
            .when(self.selected_group().is_none(), |this| {
                this.child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child("请先选择一个消费者组")
                )
            })
    }

    /// Filter groups by search
    fn filtered_groups(&self) -> Vec<&TopicConsumerGroup> {
        if self.search_query.is_empty() {
            return self.consumer_groups.iter().collect();
        }
        let query_lower = self.search_query.to_lowercase();
        self.consumer_groups.iter()
            .filter(|g| g.group_id.to_lowercase().contains(&query_lower))
            .collect()
    }
}

/// Format lag number
fn format_lag(lag: i64) -> String {
    if lag >= 1000000 {
        format!("{}M", lag / 1000000)
    } else if lag >= 1000 {
        format!("{}K", lag / 1000)
    } else {
        lag.to_string()
    }
}

impl IntoElement for TopicConsumerGroupsView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .w_full()
            .h_full()
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .p(px(12.0))
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
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
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("消费者组")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(format!("({})", self.consumer_groups.len()))
                            )
                    )
                    .child(
                        // Search
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.text_muted.opacity(0.5))
                            )
                            .child(
                                div()
                                    .text_color(if self.search_query.is_empty() {
                                        theme.text_muted
                                    } else {
                                        theme.text
                                    })
                                    .text_xs()
                                    .child(if self.search_query.is_empty() {
                                        "搜索...".to_string()
                                    } else {
                                        self.search_query.clone()
                                    })
                            )
                    )
            )
            .child(self.render_tabs())
            .child(
                div()
                    .flex_1()
                    
                    .child(match self.selected_tab {
                        ConsumerGroupTab::Groups => self.render_groups(),
                        ConsumerGroupTab::Members => self.render_members(),
                        ConsumerGroupTab::Partitions => self.render_partitions(),
                    })
            )
    }
}

impl Clone for TopicConsumerGroupsView {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            topic_name: self.topic_name.clone(),
            cluster_name: self.cluster_name.clone(),
            consumer_groups: self.consumer_groups.clone(),
            selected_group: self.selected_group,
            selected_tab: self.selected_tab,
            search_query: self.search_query.clone(),
        }
    }
}