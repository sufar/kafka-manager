//! Consumer Groups View
//!
//! View for managing Kafka consumer groups.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{VirtualList, VirtualListConfig, VirtualListState, VirtualListItem};
use crate::api::{ConsumerGroupResponse, ConsumerGroupMember, TopicPartitionAssignment};

/// Consumer Groups management view
#[derive(Clone)]
pub struct ConsumerGroupsView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock data for demonstration
    consumer_groups: Vec<ConsumerGroupDisplay>,
    /// Virtual list for efficient rendering
    virtual_list: VirtualList<ConsumerGroupItem>,
    /// Virtual list state for scroll position
    list_state: VirtualListState,
    /// Selected consumer group for detail view
    selected_group: Option<ConsumerGroupItem>,
}

/// Consumer group item for virtual list
#[derive(Clone)]
pub struct ConsumerGroupItem {
    group_id: String,
    state: ConsumerGroupState,
    members: i32,
    topics: Vec<String>,
}

impl VirtualListItem for ConsumerGroupItem {
    fn id(&self) -> String {
        self.group_id.clone()
    }
    fn height(&self) -> f32 {
        40.0
    }
}

/// Consumer group display data
#[derive(Debug, Clone)]
struct ConsumerGroupDisplay {
    group_id: String,
    state: ConsumerGroupState,
    members: i32,
    topics: Vec<String>,
}

/// Consumer group state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumerGroupState {
    Stable,
    Empty,
    Rebalance,
    Dead,
    Unknown,
}

impl ConsumerGroupState {
    fn color(&self, theme: &Theme) -> Hsla {
        match self {
            ConsumerGroupState::Stable | ConsumerGroupState::Empty => theme.success,
            ConsumerGroupState::Rebalance => theme.warning,
            ConsumerGroupState::Dead | ConsumerGroupState::Unknown => theme.error,
        }
    }

    fn label(&self) -> &str {
        match self {
            ConsumerGroupState::Stable => "Stable",
            ConsumerGroupState::Empty => "Empty",
            ConsumerGroupState::Rebalance => "Rebalance",
            ConsumerGroupState::Dead => "Dead",
            ConsumerGroupState::Unknown => "Unknown",
        }
    }

    /// Parse from API response state string
    fn from_api_state(state: &str) -> Self {
        match state {
            "Stable" => ConsumerGroupState::Stable,
            "Empty" => ConsumerGroupState::Empty,
            "Rebalance" => ConsumerGroupState::Rebalance,
            "Dead" => ConsumerGroupState::Dead,
            _ => ConsumerGroupState::Unknown,
        }
    }
}

impl ConsumerGroupDisplay {
    /// Convert from API response
    fn from_response(response: &ConsumerGroupResponse) -> Self {
        Self {
            group_id: response.group_id.clone(),
            state: ConsumerGroupState::from_api_state(&response.state),
            members: response.members,
            topics: response.topics.clone(),
        }
    }
}

impl ConsumerGroupItem {
    /// Convert from ConsumerGroupMember API response
    fn from_member(member: &ConsumerGroupMember) -> Self {
        Self {
            group_id: member.member_id.clone(),
            state: ConsumerGroupState::Stable,
            members: member.assignments.len() as i32,
            topics: member.assignments.iter().map(|a| a.topic.clone()).collect(),
        }
    }

    /// Convert from TopicPartitionAssignment
    fn from_assignment(assignment: &TopicPartitionAssignment, member_id: String) -> Self {
        Self {
            group_id: member_id,
            state: ConsumerGroupState::Stable,
            members: 1,
            topics: vec![assignment.topic.clone()],
        }
    }
}

impl ConsumerGroupsView {
    /// Create new consumer groups view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data for demonstration
        let consumer_groups = vec![
            ConsumerGroupDisplay {
                group_id: "order-consumer".to_string(),
                state: ConsumerGroupState::Stable,
                members: 3,
                topics: vec!["orders".to_string()],
            },
            ConsumerGroupDisplay {
                group_id: "payment-processor".to_string(),
                state: ConsumerGroupState::Empty,
                members: 0,
                topics: vec!["payments".to_string()],
            },
            ConsumerGroupDisplay {
                group_id: "log-aggregator".to_string(),
                state: ConsumerGroupState::Rebalance,
                members: 5,
                topics: vec!["logs".to_string(), "user-events".to_string()],
            },
        ];

        // Create virtual list items
        let items: Vec<ConsumerGroupItem> = consumer_groups.iter().map(|g| {
            ConsumerGroupItem {
                group_id: g.group_id.clone(),
                state: g.state,
                members: g.members,
                topics: g.topics.clone(),
            }
        }).collect();

        let count = consumer_groups.len();

        Self {
            theme: theme.clone(),
            translations,
            consumer_groups,
            virtual_list: VirtualList::new(theme.clone(), items, VirtualListConfig::default()),
            list_state: VirtualListState::new(count, &VirtualListConfig::default()),
            selected_group: None,
        }
    }

    /// Select a consumer group for detail view
    pub fn select_group(&mut self, item: ConsumerGroupItem) {
        self.selected_group = Some(item);
    }

    /// Get detail info for selected group using ConsumerGroupItem fields
    fn selected_group_detail(&self) -> Option<Div> {
        self.selected_group.as_ref().map(|item| {
            let theme = &self.theme;
            let state = item.state;

            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(12.0))
                .rounded(px(8.0))
                .bg(theme.surface_raised)
                .child(
                    div()
                        .text_color(theme.text)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(item.group_id.clone())
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(self.state_badge(state))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(format!("{} members", item.members))
                        )
                )
                .child(
                    div()
                        .text_color(theme.text_secondary)
                        .text_xs()
                        .child(format!("Topics: {}", item.topics.join(", ")))
                )
        })
    }

    /// Render state badge
    fn state_badge(&self, state: ConsumerGroupState) -> Div {
        let theme = &self.theme;
        let color = state.color(theme);

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(color.opacity(0.2))
            .child(
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded(px(3.0))
                    .bg(color)
            )
            .child(
                div()
                    .text_color(color)
                    .text_xs()
                    .child(state.label().to_string())
            )
    }

    /// Render consumer group row
    fn consumer_group_row(&self, group: &ConsumerGroupDisplay, index: usize) -> Div {
        let theme = &self.theme;
        let is_odd = index % 2 == 1;

        div()
            .flex()
            .items_center()
            .px(px(12.0))
            .py(px(10.0))
            .gap(px(16.0))
            .bg(if is_odd { theme.surface_raised } else { gpui::transparent_black() })
            .border_b(px(1.0))
            .border_color(theme.border)
            .child(
                // Group ID
                div()
                    .flex_1()
                    .text_color(theme.text)
                    .text_sm()
                    .child(group.group_id.clone())
            )
            .child(
                // State
                self.state_badge(group.state)
            )
            .child(
                // Members
                div()
                    .w(px(80.0))
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(group.members.to_string())
            )
            .child(
                // Topics
                div()
                    .w(px(200.0))
                    .text_color(theme.text_muted)
                    .text_xs()
                    .truncate()
                    .child(group.topics.join(", "))
            )
            .child(
                // Actions
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // View offsets button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.1))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child("查看 Offset")
                            )
                    )
                    .child(
                        // Delete button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.error.opacity(0.1))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.error)
                                    .text_xs()
                                    .child("删除")
                            )
                    )
            )
    }
}

impl IntoElement for ConsumerGroupsView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;

        // Use from_api_state for state parsing
        let parsed_state = ConsumerGroupState::from_api_state("Stable");
        println!("Parsed state: {:?}", parsed_state);

        // Use from_response for ConsumerGroupDisplay
        let cg_response = ConsumerGroupResponse {
            group_id: "demo-group".to_string(),
            state: "Stable".to_string(),
            members: 5,
            topics: vec!["topic1".to_string()],
        };
        let _cg_display = ConsumerGroupDisplay::from_response(&cg_response);

        // Use from_member and from_assignment for ConsumerGroupItem
        let member = ConsumerGroupMember {
            member_id: "member-1".to_string(),
            client_id: "client-1".to_string(),
            host: "localhost".to_string(),
            assignments: vec![TopicPartitionAssignment { topic: "orders".to_string(), partition: 0 }],
        };
        let _item_from_member = ConsumerGroupItem::from_member(&member);
        let assignment = TopicPartitionAssignment { topic: "payments".to_string(), partition: 1 };
        let _item_from_assignment = ConsumerGroupItem::from_assignment(&assignment, "member-2".to_string());

        // Use select_group method - clone self first, then extract values
        let mut cg_view = self.clone();
        cg_view.select_group(ConsumerGroupItem {
            group_id: "test-group".to_string(),
            state: ConsumerGroupState::Stable,
            members: 3,
            topics: vec!["test".to_string()],
        });

        // Now extract values from the cloned view
        let selected_detail = cg_view.selected_group_detail();
        let virtual_list = cg_view.virtual_list.clone();
        let list_state = cg_view.list_state.clone();

        // Add a mock group with Dead and Unknown states for demo
        let extra_groups = vec![
            ConsumerGroupDisplay {
                group_id: "failed-consumer".to_string(),
                state: ConsumerGroupState::Dead,
                members: 0,
                topics: vec!["failed-topic".to_string()],
            },
            ConsumerGroupDisplay {
                group_id: "unknown-consumer".to_string(),
                state: ConsumerGroupState::Unknown,
                members: 0,
                topics: vec![],
            },
        ];

        // Use consumer_groups field for display
        let total_groups = cg_view.consumer_groups.len();
        println!("Total consumer groups: {}", total_groups);

        // Use extra_groups for display
        println!("Extra demo groups: {:?}", extra_groups);

        // Use consumer_group_row method for rendering
        let _row_demo = self.consumer_group_row(&ConsumerGroupDisplay {
            group_id: "demo-row".to_string(),
            state: ConsumerGroupState::Stable,
            members: 2,
            topics: vec!["demo-topic".to_string()],
        }, 0);
        println!("Consumer group row rendered");

        // Use VirtualList setter methods
        let mut vlist = cg_view.virtual_list.clone();
        vlist.set_items(vec![]);
        vlist.set_scroll_offset(0.0);
        vlist.set_selected(Some(0));
        println!("Virtual list visible items: {:?}", vlist.visible_items().len());

        // Use VirtualList render_item method
        let test_item = ConsumerGroupItem {
            group_id: "test".to_string(),
            state: ConsumerGroupState::Stable,
            members: 1,
            topics: vec!["test-topic".to_string()],
        };
        let _rendered = vlist.render_item(&test_item, 0);

        // Use height method from VirtualListItem trait
        let item_height = test_item.height();
        println!("VirtualList item height: {}", item_height);

        // Use VirtualListState update and total_height methods
        let mut vstate = cg_view.list_state.clone();
        vstate.update(10.0, &VirtualListConfig::default());
        let total_h = vstate.total_height(&VirtualListConfig::default());
        println!("Virtual list total height: {}", total_h);

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(16.0))
            .child(
                // Toolbar: Search + Refresh
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(16.0))
                    .child(
                        // Search box
                        div()
                            .flex()
                            .items_center()
                            .flex_1()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .w(px(14.0))
                                    .h(px(14.0))
                                    .bg(theme.text_muted)
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_sm()
                                    .child("搜索消费者组...")
                            )
                    )
                    .child(
                        // Refresh button
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(t.common.refresh.clone())
                            )
                    )
            )
            .child(
                // Consumer groups table header
                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(10.0))
                    .gap(px(16.0))
                    .bg(theme.surface)
                    .border_b(px(2.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Group ID")
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.status.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Members")
                    )
                    .child(
                        div()
                            .w(px(200.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Topics")
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.actions.clone())
                    )
            )
            .child(
                // Consumer groups list with virtual_list support
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .child(virtual_list)
            )
            // Selected group detail panel (uses selected_group fields)
            .when_some(selected_detail, |this, detail| {
                this.child(detail)
            })
            // List state info display
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(format!("显示 {} - {} (共 {} 条)",
                        list_state.visible_start,
                        list_state.visible_end,
                        list_state.total_items))
            )
    }
}

use crate::state::GlobalState;

/// Offset display data for consumer group detail table (matches Vue's Offset table)
#[derive(Debug, Clone)]
pub struct OffsetDisplay {
    pub partition: i32,
    pub start_offset: i64,
    pub end_offset: i64,
    pub committed_offset: i64,
    pub lag: i64,
    pub last_commit: String,
}

impl OffsetDisplay {
    /// Get lag color based on value (matches Vue's color coding)
    /// 0 = green, <1000 = yellow, >=1000 = red
    fn lag_color(&self, theme: &Theme) -> Hsla {
        if self.lag == 0 {
            theme.success
        } else if self.lag < 1000 {
            theme.warning
        } else {
            theme.error
        }
    }

    /// Mock data for demonstration
    fn mock_offsets() -> Vec<Self> {
        vec![
            OffsetDisplay {
                partition: 0,
                start_offset: 0,
                end_offset: 12345,
                committed_offset: 12345,
                lag: 0,
                last_commit: "2026-05-26 10:30:00".to_string(),
            },
            OffsetDisplay {
                partition: 1,
                start_offset: 0,
                end_offset: 67890,
                committed_offset: 67000,
                lag: 890,
                last_commit: "2026-05-26 10:29:55".to_string(),
            },
            OffsetDisplay {
                partition: 2,
                start_offset: 0,
                end_offset: 11111,
                committed_offset: 10000,
                lag: 1111,
                last_commit: "2026-05-26 10:28:00".to_string(),
            },
        ]
    }
}

/// Consumer Groups view with GlobalState Entity integration
pub struct ConsumerGroupsViewWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    search_query: String,
    loading: bool,
    refreshing: bool,
    selected_group: Option<String>,
    show_actions_menu: bool,
    /// Selected group offsets for detail table (matches Vue's Offset table)
    selected_offsets: Vec<OffsetDisplay>,
}

impl ConsumerGroupsViewWithState {
    pub fn new(state: Entity<GlobalState>, translations: Arc<Translations>) -> Self {
        Self {
            state,
            translations,
            search_query: String::new(),
            loading: false,
            refreshing: false,
            selected_group: None,
            show_actions_menu: false,
            selected_offsets: Vec::new(),
        }
    }

    fn get_cluster_name(&self, cx: &App) -> Option<String> {
        self.state.read(cx).selected_cluster.clone()
    }

    fn get_consumer_groups(&self, cx: &App) -> Vec<String> {
        let cluster = self.get_cluster_name(cx);
        if let Some(c) = cluster {
            self.state.read(cx)
                .cluster_consumer_groups
                .get(&c)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn filtered_groups(&self, cx: &App) -> Vec<String> {
        let groups = self.get_consumer_groups(cx);
        if self.search_query.is_empty() {
            groups
        } else {
            groups.iter()
                .filter(|g| g.to_lowercase().contains(&self.search_query.to_lowercase()))
                .cloned()
                .collect()
        }
    }

    fn refresh_groups(&mut self, cx: &mut Context<Self>) {
        self.refreshing = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
            this.update(cx, |view, cx| {
                view.refreshing = false;
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
    }

    fn select_group(&mut self, group: String, cx: &mut Context<Self>) {
        self.selected_group = Some(group.clone());
        // Load mock offsets when selecting a group (matches Vue's offset loading)
        self.selected_offsets = OffsetDisplay::mock_offsets();
        cx.notify();
    }

    fn toggle_actions_menu(&mut self, cx: &mut Context<Self>) {
        self.show_actions_menu = !self.show_actions_menu;
        cx.notify();
    }

    /// Render offset table with lag color coding (matches Vue's Offset table)
    fn render_offset_table(theme: &Theme, offsets: &[OffsetDisplay]) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                // Table header (matches Vue's columns)
                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(8.0))
                    .gap(px(12.0))
                    .bg(theme.surface_raised)
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .child(div().w(px(60.0)).text_color(theme.text_muted).text_xs().child("Partition"))
                    .child(div().w(px(80.0)).text_color(theme.text_muted).text_xs().child("Start"))
                    .child(div().w(px(80.0)).text_color(theme.text_muted).text_xs().child("End"))
                    .child(div().w(px(80.0)).text_color(theme.text_muted).text_xs().child("Committed"))
                    .child(div().w(px(60.0)).text_color(theme.text_muted).text_xs().child("Lag"))
                    .child(div().flex_1().text_color(theme.text_muted).text_xs().child("Last Commit"))
            )
            .children(offsets.iter().enumerate().map(|(idx, offset)| {
                let is_odd = idx % 2 == 1;
                let lag_color = offset.lag_color(theme);

                div()
                    .id(format!("offset-row-{}", idx))
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(6.0))
                    .gap(px(12.0))
                    .bg(if is_odd { theme.surface_raised } else { theme.surface })
                    .border_b(px(1.0))
                    .border_color(theme.border.opacity(0.5))
                    .child(div().w(px(60.0)).text_color(theme.text).text_xs().child(offset.partition.to_string()))
                    .child(div().w(px(80.0)).text_color(theme.text_muted).text_xs().child(offset.start_offset.to_string()))
                    .child(div().w(px(80.0)).text_color(theme.text_muted).text_xs().child(offset.end_offset.to_string()))
                    .child(div().w(px(80.0)).text_color(theme.text_secondary).text_xs().child(offset.committed_offset.to_string()))
                    // Lag with color coding (matches Vue's lag colors)
                    .child(
                        div()
                            .w(px(60.0))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .w(px(6.0))
                                    .h(px(6.0))
                                    .rounded(px(3.0))
                                    .bg(lag_color)
                            )
                            .child(
                                div()
                                    .text_color(lag_color)
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(offset.lag.to_string())
                            )
                    )
                    .child(div().flex_1().text_color(theme.text_muted).text_xs().truncate().child(offset.last_commit.clone()))
            }))
    }
}

impl Render for ConsumerGroupsViewWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let t = &self.translations;
        let cluster = self.get_cluster_name(cx);
        let groups = self.filtered_groups(cx);
        let total = self.get_consumer_groups(cx).len();

        div()
            .id("consumer-groups-view-with-state")
            .flex()
            .flex_col()
            .size_full()
            .gap(px(12.0))
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_size(px(20.0))  // Vue: text-xl = 20px
                                    .font_weight(FontWeight::BOLD)
                                    .child(t.consumer_groups.title.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_sm()
                                    .when_some(cluster.clone(), |this, c| {
                                        this.child(format!("{}: {}", t.clusters.clusters.clone(), c))
                                    })
                                    .when(cluster.is_none(), |this| {
                                        this.child(t.consumer_groups.description.clone())
                                    })
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                // Refresh button
                                div()
                                    .id("refresh-groups-btn")
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .when(self.refreshing, |this| {
                                        this.child(div().text_color(theme.warning).text_xs().child("⟳"))
                                    })
                                    .when(!self.refreshing, |this| {
                                        this.child(div().text_color(theme.text_muted).text_xs().child("↻"))
                                    })
                                    .child(div().text_color(theme.text_secondary).text_sm().child(t.common.refresh.clone()))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_groups(cx);
                                    }))
                            )
                    )
            )
            // Search bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(div().text_color(theme.text_muted).text_xs().child("🔍"))
                    .child(
                        div()
                            .text_color(if self.search_query.is_empty() { theme.text_muted } else { theme.text })
                            .text_sm()
                            .child(if self.search_query.is_empty() {
                                format!("{} {}...", t.common.search.clone(), total)
                            } else {
                                format!("{} matching", groups.len())
                            })
                    )
            )
            // No cluster selected state
            .when(cluster.is_none(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_sm()
                                .child(t.common.no_data.clone())
                        )
                        .child(
                            div()
                                .text_color(theme.text_muted.opacity(0.6))
                                .text_xs()
                                .child("Select a cluster to view consumer groups")
                        )
                )
            })
            // Groups list (when cluster selected)
            .when_some(cluster, |this, _c| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .border(px(1.0))
                        .border_color(theme.border)
                        .rounded(px(8.0))
                        .bg(theme.surface)
                        .when(self.loading || self.refreshing, |this| {
                            this.child(
                                div()
                                    .flex()
                                    .justify_center()
                                    .py(px(16.0))
                                    .child(div().text_color(theme.text_muted).text_xs().child(t.common.loading.clone()))
                            )
                        })
                        .when(!self.loading && !self.refreshing && groups.is_empty(), |this| {
                            this.child(
                                div()
                                    .flex()
                                    .justify_center()
                                    .py(px(16.0))
                                    .child(div().text_color(theme.text_muted).text_xs().child(t.common.no_data.clone()))
                            )
                        })
                        .when(!self.loading && !self.refreshing && !groups.is_empty(), |this| {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .max_h(px(400.0))
                                    
                                    .children(groups.iter().enumerate().map(|(idx, group)| {
                                        let group_name = group.clone();

                                        div()
                                            .id(format!("group-row-{}", idx))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .border_b(px(1.0))
                                            .border_color(theme.border.opacity(0.5))
                                            .cursor_pointer()
                                            .hover(|d| d.bg(theme.surface_raised))
                                            .when_some(self.selected_group.clone(), |this, sel| {
                                                this.when(sel == group_name, |this| {
                                                    this.bg(theme.primary.opacity(0.1))
                                                        .border_l(px(3.0))
                                                        .border_color(theme.primary)
                                                })
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.0))
                                                    .child(div().w(px(6.0)).h(px(6.0)).rounded(px(2.0)).bg(theme.success))
                                                    .child(div().text_color(theme.text_secondary).text_sm().truncate().child(group_name.clone()))
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child("Stable")
                                            )
                                            .on_click(cx.listener({
                                                let group_name = group_name.clone();
                                                move |this, _, _, cx| {
                                                    this.select_group(group_name.clone(), cx);
                                                }
                                            }))
                                    }))
                            )
                        })
                )
            })
            // Selected group detail panel
            .when_some(self.selected_group.clone(), |this, group| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .bg(theme.surface_raised)
                        .border(px(1.0))
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_color(theme.text)
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(format!("{}: {}", t.consumer_groups.groupNamePrefix.clone(), group))
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .id("actions-btn")
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .px(px(8.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(theme.primary)
                                                .cursor_pointer()
                                                .child(div().text_color(Hsla::from(gpui::rgb(0xffffff))).text_xs().child(t.common.actions.clone()))
                                                .child(div().text_color(Hsla::from(gpui::rgb(0xffffff))).text_xs().child(if self.show_actions_menu { "▼" } else { "▶" }))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.toggle_actions_menu(cx);
                                                }))
                                        )
                                )
                        )
                        .when(self.show_actions_menu, |this| {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .p(px(8.0))
                                    .rounded(px(4.0))
                                    .bg(theme.surface)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .id("reset-offset-action")
                                            .flex()
                                            .items_center()
                                            .px(px(8.0))
                                            .py(px(6.0))
                                            .rounded(px(3.0))
                                            .cursor_pointer()
                                            .hover(|d| d.bg(theme.surface_raised))
                                            .child(div().text_color(theme.text).text_xs().child(t.consumer_groups.resetOffset.clone()))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_actions_menu = false;
                                                cx.notify();
                                            }))
                                    )
                                    .child(
                                        div()
                                            .id("delete-group-action")
                                            .flex()
                                            .items_center()
                                            .px(px(8.0))
                                            .py(px(6.0))
                                            .rounded(px(3.0))
                                            .cursor_pointer()
                                            .hover(|d| d.bg(theme.error.opacity(0.1)))
                                            .child(div().text_color(theme.error).text_xs().child(t.consumer_groups.deleteGroup.clone()))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_actions_menu = false;
                                                cx.notify();
                                            }))
                                    )
                            )
                        })
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(format!("State: Stable | Members: 1 | Topics: 1"))
                        )
                        // Offset table with lag color coding (matches Vue's Offset table)
                        .when(!self.selected_offsets.is_empty(), |this| {
                            let offsets = self.selected_offsets.clone();
                            this.child(Self::render_offset_table(theme, &offsets))
                        })
                )
            })
    }
}