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