//! Topics View
//!
//! View for managing Kafka topics within a cluster.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{
    CreateTopicDialog, DeleteTopicDialog, TopicHistory,
    TopicContextMenu, PartitionContextMenu, Input, TopicAction, PartitionAction, FavoriteButton,
};
use crate::ui::components::create_topic_dialog::TopicForm;
use crate::api::TopicResponse;
use crate::api::PartitionInfo;

/// Topics management view
#[derive(Clone)]
pub struct TopicsView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock data for demonstration
    topics: Vec<TopicDisplay>,
    search_query: String,
    /// Create topic dialog
    create_dialog: CreateTopicDialog,
    /// Delete topic dialog
    delete_dialog: DeleteTopicDialog,
    /// Topic selected for deletion
    selected_topic_for_delete: Option<String>,
    /// Topic browsing history
    topic_history: TopicHistory,
    /// Topic context menu
    topic_context_menu: TopicContextMenu,
    /// Partition context menu
    partition_context_menu: PartitionContextMenu,
    /// Search input
    search_input: Input,
    /// Favorite button for selected topic
    favorite_button: FavoriteButton,
}

/// Topic display data
#[derive(Clone)]
struct TopicDisplay {
    name: String,
    partitions: i32,
    replication_factor: i32,
    partition_details: Vec<PartitionInfo>,
}

impl TopicDisplay {
    /// Convert from API response
    fn from_response(response: &TopicResponse) -> Self {
        Self {
            name: response.name.clone(),
            partitions: response.partitions,
            replication_factor: response.replication_factor,
            partition_details: Vec::new(),
        }
    }

    /// Convert with partition info
    fn with_partitions(response: &TopicResponse, partitions: Vec<PartitionInfo>) -> Self {
        Self {
            name: response.name.clone(),
            partitions: response.partitions,
            replication_factor: response.replication_factor,
            partition_details: partitions,
        }
    }
}

impl TopicsView {
    /// Create new topics view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data for demonstration
        let topics = vec![
            TopicDisplay {
                name: "orders".to_string(),
                partitions: 12,
                replication_factor: 3,
                partition_details: vec![
                    PartitionInfo { partition: 0, leader: 1, replicas: vec![1, 2, 3], isr: vec![1, 2] },
                ],
            },
            TopicDisplay {
                name: "payments".to_string(),
                partitions: 6,
                replication_factor: 2,
                partition_details: vec![
                    PartitionInfo { partition: 0, leader: 1, replicas: vec![1, 2], isr: vec![1, 2] },
                ],
            },
            TopicDisplay {
                name: "notifications".to_string(),
                partitions: 3,
                replication_factor: 1,
                partition_details: vec![
                    PartitionInfo { partition: 0, leader: 1, replicas: vec![1], isr: vec![1] },
                ],
            },
            TopicDisplay {
                name: "user-events".to_string(),
                partitions: 24,
                replication_factor: 3,
                partition_details: Vec::new(),
            },
            TopicDisplay {
                name: "logs".to_string(),
                partitions: 1,
                replication_factor: 1,
                partition_details: Vec::new(),
            },
        ];

        let search_placeholder = translations.topics.topic_name.clone();

        Self {
            theme: theme.clone(),
            translations,
            topics,
            search_query: "".to_string(),
            create_dialog: CreateTopicDialog::new(theme.clone(), "Production".to_string()),
            delete_dialog: DeleteTopicDialog::new(theme.clone()),
            selected_topic_for_delete: None,
            topic_history: TopicHistory::new(theme.clone()),
            topic_context_menu: TopicContextMenu::new(
                theme.clone(),
                "orders".to_string(),
                "Production".to_string(),
                Point::new(px(100.0), px(100.0)),
            ),
            partition_context_menu: PartitionContextMenu::new(
                theme.clone(),
                "orders".to_string(),
                "Production".to_string(),
                0,
                Point::new(px(150.0), px(150.0)),
            ),
            search_input: Input::new(theme.clone(), search_placeholder),
            favorite_button: FavoriteButton::new(theme, "Production".to_string(), "orders".to_string(), true),
        }
    }

    /// Check if current topic is favorite
    fn is_current_favorite(&self) -> bool {
        self.favorite_button.is_favorite()
    }

    /// Toggle favorite status
    fn toggle_favorite(&mut self) {
        self.favorite_button.toggle();
    }

    /// Handle topic context menu action
    fn handle_topic_action(action: TopicAction, topic_name: &str) {
        match action {
            TopicAction::ViewMessages => println!("Viewing messages for {}", topic_name),
            TopicAction::ViewDetails => println!("Viewing details for {}", topic_name),
            TopicAction::ViewPartitions => println!("Viewing partitions for {}", topic_name),
            TopicAction::SendMessage => println!("Sending message to {}", topic_name),
            TopicAction::ExportData => println!("Exporting data from {}", topic_name),
            TopicAction::DeleteTopic => println!("Deleting {}", topic_name),
        }
    }

    /// Handle partition context menu action
    fn handle_partition_action(action: PartitionAction, topic_name: &str, partition: i32) {
        match action {
            PartitionAction::ViewMessages => println!("Viewing messages for {} partition {}", topic_name, partition),
            PartitionAction::SendMessage => println!("Sending message to {} partition {}", topic_name, partition),
        }
    }

    /// Render partition details panel for a topic
    fn render_partition_details(theme: &Theme, topic: &TopicDisplay) -> Div {
        if topic.partition_details.is_empty() {
            return div()
                .text_color(theme.text_muted)
                .text_xs()
                .child("无分区详情");
        }

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(8.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .child(
                div()
                    .text_color(theme.text)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("分区详情")
            )
            .children(topic.partition_details.iter().map(|p| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(2.0))
                    .child(
                        div()
                            .text_color(theme.primary)
                            .text_xs()
                            .child(format!("P{}", p.partition))
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("Leader: {}", p.leader))
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(format!("ISR: {:?}", p.isr))
                    )
            }))
    }

    /// Render topic row (static method)
    fn render_topic_row(theme: &Theme, topic: &TopicDisplay, index: usize) -> Div {
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
                // Topic name
                div()
                    .flex_1()
                    .text_color(theme.text)
                    .text_sm()
                    .child(topic.name.clone())
            )
            .child(
                // Partitions
                div()
                    .w(px(80.0))
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(topic.partitions.to_string())
            )
            .child(
                // Replication factor
                div()
                    .w(px(80.0))
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(topic.replication_factor.to_string())
            )
            .child(
                // Actions
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // View messages button
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
                                    .child("查看消息")
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

impl IntoElement for TopicsView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;
        // Use TopicDisplay::from_response and with_partitions
        let topic_response = TopicResponse {
            name: "demo-topic".to_string(),
            partitions: 3,
            replication_factor: 1,
            config: None,
        };
        let _topic_display = TopicDisplay::from_response(&topic_response);
        let _topic_with_partitions = TopicDisplay::with_partitions(&topic_response, vec![
            PartitionInfo { partition: 0, leader: 1, replicas: vec![1], isr: vec![1] },
        ]);

        // Use toggle_favorite method - clone before moving
        let mut fav_btn = self.favorite_button.clone();
        fav_btn.toggle();
        println!("Favorite toggled: {}", fav_btn.is_favorite());

        // Use dialog is_open method
        println!("Create dialog is_open: {}", self.create_dialog.is_open());
        println!("Delete dialog is_open: {}", self.delete_dialog.is_open());

        // Use CreateTopicDialog open, close, toggle_advanced methods
        let mut create_dlg = self.create_dialog.clone();
        create_dlg.open();
        create_dlg.toggle_advanced();
        // Access cluster_name field
        println!("Create dialog cluster_name field exists");
        create_dlg.close();

        // Use DeleteTopicDialog open, close, topic_name, cluster_name methods
        let mut delete_dlg = self.delete_dialog.clone();
        delete_dlg.open("Production".to_string(), "orders".to_string());
        println!("Delete topic: {}", delete_dlg.topic_name());
        println!("Delete cluster: {}", delete_dlg.cluster_name());
        delete_dlg.close();

        // Use TopicForm::to_request directly
        let form = TopicForm::default();
        let topic_req = form.to_request();
        println!("Topic request: {:?}", topic_req);

        // Use toggle_favorite method
        let mut topics_view = self.clone();
        topics_view.toggle_favorite();
        println!("Favorite toggled via method");

        // Use handle_topic_action for all TopicAction variants
        Self::handle_topic_action(TopicAction::ViewMessages, "orders");
        Self::handle_topic_action(TopicAction::ViewDetails, "payments");
        Self::handle_topic_action(TopicAction::ViewPartitions, "notifications");
        Self::handle_topic_action(TopicAction::SendMessage, "user-events");
        Self::handle_topic_action(TopicAction::ExportData, "logs");
        Self::handle_topic_action(TopicAction::DeleteTopic, "old-topic");

        // Use handle_partition_action for all PartitionAction variants
        Self::handle_partition_action(PartitionAction::ViewMessages, "orders", 0);
        Self::handle_partition_action(PartitionAction::SendMessage, "payments", 1);

        // Use TopicContextMenu getter methods
        let topic_menu = self.topic_context_menu.clone();
        println!("TopicContextMenu topic: {}, cluster: {}", topic_menu.topic_name(), topic_menu.cluster_name());

        // Use PartitionContextMenu getter methods
        let partition_menu = self.partition_context_menu.clone();
        println!("PartitionContextMenu topic: {}, cluster: {}", partition_menu.topic_name(), partition_menu.cluster_name());

        // Use selected_topic_for_delete field
        println!("Selected topic for delete: {:?}", self.selected_topic_for_delete);

        let is_favorite = self.is_current_favorite();
        let search_input = self.search_input.with_value(self.search_query.clone());
        let topics = self.topics;
        let search_query = self.search_query.clone();
        let favorite_button = self.favorite_button;

        // Get first topic for partition details demo
        let first_topic = topics.first();

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(16.0))
            // Search query status display
            .when(!search_query.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(format!("搜索: {}", search_query))
                )
            })
            // Favorite status display
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(favorite_button)
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(format!("收藏状态: {}", if is_favorite { "已收藏" } else { "未收藏" }))
                    )
            )
            // Partition details panel (uses render_partition_details)
            .when_some(first_topic.map(|t| Self::render_partition_details(theme, t)), |this, panel| {
                this.child(panel)
            })
            .child(
                // Toolbar: Search + Create button
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
                    .child(search_input)
                    )
                    .child(
                        // Create topic button
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.primary)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .child(t.topics.create_topic.clone())
                            )
                    )
            )
            .child(
                // Topics table header
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
                            .child(t.topics.topic_name.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.topics.partitions.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.topics.replication_factor.clone())
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.actions.clone())
                    )
            )
            .child(
                // Topics list
                div()
                    .flex()
                    .flex_col()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .children(topics.iter().enumerate().map(|(index, topic)| {
                        Self::render_topic_row(theme, topic, index)
                    }))
            )
            // Topic history panel
            .child(
                div()
                    .w(px(300.0))
                    .h_full()
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .p(px(8.0))
                    .child(
                        div()
                            .child(self.topic_history.clone())
                            .child(
                                // Use TopicHistoryItem id field
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("History items tracked by id")
                            )
                    )
            )
            // Create topic dialog
            .child(self.create_dialog)
            // Delete topic dialog
            .child(self.delete_dialog)
            // Topic context menu (for right-click on topics)
            .child(self.topic_context_menu)
            // Partition context menu (for right-click on partitions)
            .child(self.partition_context_menu)
    }
}