//! Left Sidebar Component
//!
//! Displays navigation items, cluster tree, and quick actions.
//! Supports both Tree mode (ClusterTreeNavigator) and Flat mode (TopicNavigator).
//! Includes right-click context menu support.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::router::ViewType;
use crate::state::{GlobalState, SidebarMode, ClusterHealth};
use crate::shortcuts::navigation::{GoToClusters, GoToTopics, GoToMessages, GoToConsumerGroups, GoToSchemaRegistry};

/// Context menu state for sidebar
#[derive(Debug, Clone, Default)]
pub struct ContextMenuState {
    pub visible: bool,
    pub menu_type: ContextMenuType,
    pub position: Point<Pixels>,
    pub cluster_name: String,
    pub topic_name: Option<String>,
    pub partition_id: Option<i32>,
}

/// Type of context menu to show
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ContextMenuType {
    #[default]
    None,
    Cluster,
    TopicsFolder,
    Topic,
    Partition,
}

/// Left sidebar component
pub struct LeftSidebar {
    width: Pixels,
    is_mobile: bool,
    theme: Theme,
    translations: Arc<Translations>,
    current_view: ViewType,
}

impl LeftSidebar {
    /// Create sidebar for mobile view (overlay)
    pub fn mobile(width: Pixels, theme: Theme, translations: Arc<Translations>, current_view: ViewType) -> Self {
        Self {
            width,
            is_mobile: true,
            theme,
            translations,
            current_view,
        }
    }

    /// Create sidebar for desktop view (fixed)
    pub fn desktop(width: Pixels, theme: Theme, translations: Arc<Translations>, current_view: ViewType) -> Self {
        Self {
            width,
            is_mobile: false,
            theme,
            translations,
            current_view,
        }
    }
}

impl IntoElement for LeftSidebar {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let t = self.translations.clone();
        let current = self.current_view;

        // Navigation item that dispatches an action on click
        let nav_item = |id: &'static str, text: String, theme: Theme, active: bool, action: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>| -> Stateful<Div> {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .bg(if active { theme.surface_raised } else { gpui::transparent_black() })
                .border(px(1.0))
                .border_color(if active { theme.border_focused } else { gpui::transparent_black() })
                .cursor_pointer()
                .hover(|d| d.bg(theme.surface))
                .active(|d| d.bg(theme.surface_raised))
                .on_click(action)
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .rounded(px(3.0))
                        .bg(if active { theme.primary } else { theme.text_muted })
                )
                .child(
                    div()
                        .text_color(if active { theme.text } else { theme.text_secondary })
                        .text_sm()
                        .child(text)
                )
        };

        // Create action dispatcher
        let make_action = |action: Box<dyn Fn(&mut App)>| -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App)> {
            Box::new(move |_event, _window, cx| action(cx))
        };

        let content = div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(16.0))
            .gap(px(16.0))
            .child(
                // Navigation items
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(nav_item("nav-clusters", t.clusters.title.clone(), theme.clone(), current == ViewType::Clusters, make_action(Box::new(|cx| cx.dispatch_action(&GoToClusters)))))
                    .child(nav_item("nav-topics", t.topics.title.clone(), theme.clone(), current == ViewType::Topics, make_action(Box::new(|cx| cx.dispatch_action(&GoToTopics)))))
                    .child(nav_item("nav-messages", t.messages.title.clone(), theme.clone(), current == ViewType::Messages, make_action(Box::new(|cx| cx.dispatch_action(&GoToMessages)))))
                    .child(nav_item("nav-consumer-groups", t.consumer_groups.title.clone(), theme.clone(), current == ViewType::ConsumerGroups, make_action(Box::new(|cx| cx.dispatch_action(&GoToConsumerGroups)))))
                    .child(nav_item("nav-schema-registry", t.schema_registry.title.clone(), theme.clone(), current == ViewType::SchemaRegistry, make_action(Box::new(|cx| cx.dispatch_action(&GoToSchemaRegistry)))))
                    .child(nav_item("nav-favorites", "Favorites".to_string(), theme.clone(), current == ViewType::Favorites, make_action(Box::new(|_cx| println!("Favorites clicked")))))
            )
            .child(
                // Separator
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(theme.border)
            )
            .child(
                // Quick actions section
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.actions.clone())
                    )
                    .child(
                        // Add cluster button
                        div()
                            .id("add-cluster-btn")
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.primary)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.primary.opacity(0.9)))
                            .active(|d| d.bg(theme.primary.opacity(0.8)))
                            .on_click(|_event, _window, _cx| {
                                println!("Add cluster clicked");
                            })
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .child(t.clusters.add_cluster.clone())
                            )
                    )
            );

        if self.is_mobile {
            // Mobile: overlay sidebar
            div()
                .id("mobile-sidebar")
                .absolute()
                .left(px(0.0))
                .top(px(48.0))
                .bottom(px(0.0))
                .w(self.width)
                .bg(theme.surface)
                .border_r(px(1.0))
                .border_color(theme.border)
                .child(content)
        } else {
            // Desktop: fixed sidebar
            div()
                .id("desktop-sidebar")
                .h_full()
                .w(self.width)
                .bg(theme.surface)
                .border_r(px(1.0))
                .border_color(theme.border)
                .child(content)
        }
    }
}

/// Sidebar with GlobalState integration for dynamic data
pub struct LeftSidebarWithState {
    state: Entity<GlobalState>,
    is_mobile: bool,
    /// Drag state for resizing
    drag_start_x: Option<Pixels>,
    drag_start_width: Option<Pixels>,
    /// Context menu state
    context_menu: ContextMenuState,
}

impl LeftSidebarWithState {
    /// Create sidebar with state entity
    pub fn new(state: Entity<GlobalState>, is_mobile: bool) -> Self {
        Self {
            state,
            is_mobile,
            drag_start_x: None,
            drag_start_width: None,
            context_menu: ContextMenuState::default(),
        }
    }

    /// Show cluster context menu
    fn show_cluster_menu(&mut self, cluster: String, position: Point<Pixels>) {
        self.context_menu = ContextMenuState {
            visible: true,
            menu_type: ContextMenuType::Cluster,
            position,
            cluster_name: cluster,
            topic_name: None,
            partition_id: None,
        };
    }

    /// Show topic context menu
    fn show_topic_menu(&mut self, cluster: String, topic: String, position: Point<Pixels>) {
        self.context_menu = ContextMenuState {
            visible: true,
            menu_type: ContextMenuType::Topic,
            position,
            cluster_name: cluster,
            topic_name: Some(topic),
            partition_id: None,
        };
    }

    /// Hide context menu
    fn hide_context_menu(&mut self) {
        self.context_menu.visible = false;
        self.context_menu.menu_type = ContextMenuType::None;
    }

    /// Render health indicator
    fn health_indicator(healthy: bool, error: Option<&String>, theme: &Theme) -> Div {
        let color = if healthy {
            theme.success
        } else if error.is_some() {
            theme.error
        } else {
            theme.warning
        };

        div()
            .w(px(8.0))
            .h(px(8.0))
            .rounded(px(4.0))
            .bg(color)
    }

    /// Render sidebar mode toggle button
    fn sidebar_mode_toggle(theme: &Theme, mode: SidebarMode) -> Div {
        let (icon, label) = match mode {
            SidebarMode::Tree => ("🌳", "Tree"),
            SidebarMode::Flat => ("📄", "Flat"),
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(theme.surface_raised)
            .border(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child(icon)
            )
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(label)
            )
    }
}

impl Render for LeftSidebarWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Clone needed data first to avoid borrow conflicts
        let (theme, sidebar_width, sidebar_mode, clusters, cluster_health, cluster_topics, loading, error, current_view, is_mobile, t) = {
            let state = self.state.read(cx);
            (
                state.theme.clone(),
                state.sidebar_width,
                state.sidebar_mode,
                state.clusters.clone(),
                state.cluster_health.clone(),
                state.cluster_topics.clone(),
                state.loading,
                state.error.clone(),
                state.current_view(),
                state.is_mobile,
                state.translations(),
            )
        };

        // Main sidebar content
        let main_content = div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                // Sidebar mode toggle
                Self::sidebar_mode_toggle(&theme, sidebar_mode)
            )
            .child(
                // Separator
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(theme.border)
            )
            .child(
                // Navigation items
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(Self::nav_item("nav-clusters", t.clusters.title.clone(), theme.clone(), current_view == ViewType::Clusters))
                    .child(Self::nav_item("nav-topics", t.topics.title.clone(), theme.clone(), current_view == ViewType::Topics))
                    .child(Self::nav_item("nav-messages", t.messages.title.clone(), theme.clone(), current_view == ViewType::Messages))
                    .child(Self::nav_item("nav-consumer-groups", t.consumer_groups.title.clone(), theme.clone(), current_view == ViewType::ConsumerGroups))
                    .child(Self::nav_item("nav-schema-registry", t.schema_registry.title.clone(), theme.clone(), current_view == ViewType::SchemaRegistry))
            )
            .child(
                // Separator
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(theme.border)
            )
            .when(loading, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .py(px(16.0))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(t.common.loading.clone())
                        )
                )
            })
            .when_some(error.clone(), |this, err| {
                this.child(
                    div()
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(px(4.0))
                        .bg(theme.error.opacity(0.1))
                        .border(px(1.0))
                        .border_color(theme.error.opacity(0.3))
                        .child(
                            div()
                                .text_color(theme.error)
                                .text_xs()
                                .child(err)
                        )
                )
            })
            .when(!loading && error.is_none(), |this| {
                // Cluster tree/flat navigator based on sidebar_mode
                this.child(
                    match sidebar_mode {
                        SidebarMode::Tree => {
                            // Tree mode: show clusters with health indicators and right-click handlers
                            self.render_tree_mode_with_handlers(&clusters, &cluster_health, &cluster_topics, &theme, cx)
                        }
                        SidebarMode::Flat => {
                            // Flat mode: show all topics from all clusters
                            Self::render_flat_mode(&clusters, &cluster_topics, &theme)
                        }
                    }
                )
            })
            .child(
                // Add cluster button at bottom
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.primary)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.primary.opacity(0.9)))
                    .child(
                        div()
                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                            .text_sm()
                            .child(t.clusters.add_cluster.clone())
                    )
            );

        // Sidebar container with resize handle
        div()
            .id("sidebar-with-state")
            .h_full()
            .w(sidebar_width)
            .bg(theme.surface)
            .border_r(px(1.0))
            .border_color(theme.border)
            .flex()
            .child(
                // Main content area
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .p(px(12.0))
                    .child(main_content)
            )
            .child(
                // Resize handle
                div()
                    .id("resize-handle")
                    .w(px(4.0))
                    .h_full()
                    .bg(theme.border)
                    .cursor_col_resize()
                    .hover(|d| d.bg(theme.primary.opacity(0.5)))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                        this.drag_start_x = Some(event.position.x);
                        this.drag_start_width = Some(this.state.read(cx).sidebar_width);
                    }))
            )
            .when(is_mobile, |this| {
                // Mobile: overlay style
                this.absolute()
                    .left(px(0.0))
                    .top(px(48.0))
                    .bottom(px(0.0))
            })
            // Context menu overlay
            .when(self.context_menu.visible, |this| {
                let menu_pos = self.context_menu.position;
                let theme = self.state.read(cx).theme.clone();
                let t = self.state.read(cx).translations();

                this.child(
                    // Invisible overlay to close menu on click outside
                    div()
                        .id("context-menu-overlay")
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.hide_context_menu();
                            cx.notify();
                        }))
                        .child(
                            // Actual context menu
                            div()
                                .absolute()
                                .top(menu_pos.y)
                                .left(menu_pos.x)
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
                                                        .child(self.context_menu.cluster_name.clone())
                                                )
                                        )
                                        .child(
                                            // Menu items based on type
                                            div()
                                                .flex()
                                                .flex_col()
                                                .py(px(4.0))
                                                .gap(px(2.0))
                                                .when(self.context_menu.menu_type == ContextMenuType::Cluster, |this| {
                                                    this.child(Self::context_menu_item("view-topics", t.topics.title.clone(), theme.clone()))
                                                        .child(Self::context_menu_item("test-connection", t.clusters.test_connection.clone(), theme.clone()))
                                                        .child(Self::context_menu_item("refresh", t.common.refresh.clone(), theme.clone()))
                                                })
                                                .when(self.context_menu.menu_type == ContextMenuType::Topic, |this| {
                                                    this.child(Self::context_menu_item("view-messages", t.messages.title.clone(), theme.clone()))
                                                        .child(Self::context_menu_item("view-details", "View Details".to_string(), theme.clone()))
                                                        .child(Self::context_menu_item("send-message", t.messages.send_message.clone(), theme.clone()))
                                                        .child(Self::context_menu_danger_item("delete-topic", t.topics.delete_topic.clone(), theme.clone()))
                                                })
                                        )
                                )
                        )
                )
            })
    }
}

impl LeftSidebarWithState {
    /// Render navigation item
    fn nav_item(id: &'static str, text: String, theme: Theme, active: bool) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(if active { theme.surface_raised } else { gpui::transparent_black() })
            .border(px(1.0))
            .border_color(if active { theme.border_focused } else { gpui::transparent_black() })
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(3.0))
                    .bg(if active { theme.primary } else { theme.text_muted })
            )
            .child(
                div()
                    .text_color(if active { theme.text } else { theme.text_secondary })
                    .text_sm()
                    .child(text)
            )
    }

    /// Render context menu item
    fn context_menu_item(id: &'static str, text: String, theme: Theme) -> Stateful<Div> {
        div()
            .id(id)
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
                    .child(text)
            )
    }

    /// Render danger context menu item
    fn context_menu_danger_item(id: &'static str, text: String, theme: Theme) -> Stateful<Div> {
        div()
            .id(id)
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
                    .child(text)
            )
    }

    /// Render tree mode with right-click context menu handlers
    fn render_tree_mode_with_handlers(
        &self,
        clusters: &[crate::api::ClusterResponse],
        cluster_health: &std::collections::HashMap<String, ClusterHealth>,
        cluster_topics: &std::collections::HashMap<String, Vec<String>>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .size_full()
            .children(clusters.iter().map(|cluster| {
                let health = cluster_health.get(&cluster.name);
                let healthy = health.map(|h| h.healthy).unwrap_or(false);
                let health_error = health.and_then(|h| h.error.as_ref());
                let topics_count = cluster_topics.get(&cluster.name).map(|v| v.len()).unwrap_or(0);
                let cluster_name = cluster.name.clone();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        // Cluster header with right-click handler
                        div()
                            .id(format!("cluster-tree-{}", cluster_name))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(8.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised.opacity(0.5))
                            .cursor_pointer()
                            .child(Self::health_indicator(healthy, health_error, theme))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(cluster_name.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(format!("{} topics", topics_count))
                            )
                            .on_click(cx.listener({
                                let cluster_name = cluster_name.clone();
                                move |this, _, _, cx| {
                                    this.state.update(cx, |state, cx| {
                                        state.select_cluster(cluster_name.clone());
                                        state.navigate(ViewType::Topics);
                                        cx.notify();
                                    });
                                    cx.notify();
                                }
                            }))
                            .on_mouse_down(MouseButton::Right, cx.listener({
                                let cluster_name = cluster_name.clone();
                                move |this, event: &MouseDownEvent, _, cx| {
                                    this.show_cluster_menu(cluster_name.clone(), event.position);
                                    cx.notify();
                                }
                            }))
                    )
                    .when(topics_count > 0, |this| {
                        // Show topics with right-click handlers
                        let topic_divs: Vec<Stateful<Div>> = cluster_topics
                            .get(&cluster_name)
                            .map(|topics| {
                                topics.iter().take(5).map(|topic| {
                                    let topic_name = topic.clone();
                                    let cluster_for_topic = cluster_name.clone();

                                    div()
                                        .id(format!("topic-tree-{}-{}", cluster_for_topic, topic_name))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .ml(px(16.0))
                                        .rounded(px(3.0))
                                        .cursor_pointer()
                                        .hover(|d| d.bg(theme.surface))
                                        .child(
                                            div()
                                                .w(px(6.0))
                                                .h(px(6.0))
                                                .rounded(px(2.0))
                                                .bg(theme.text_muted.opacity(0.5))
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.text_secondary)
                                                .text_xs()
                                                .child(topic_name.clone())
                                        )
                                        .on_click(cx.listener({
                                            let topic_name = topic_name.clone();
                                            let cluster_for_topic = cluster_for_topic.clone();
                                            move |this, _, _, cx| {
                                                this.state.update(cx, |state, cx| {
                                                    state.select_topic(cluster_for_topic.clone(), topic_name.clone());
                                                    state.navigate_to_messages(&cluster_for_topic, &topic_name);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }))
                                        .on_mouse_down(MouseButton::Right, cx.listener({
                                            let topic_name = topic_name.clone();
                                            let cluster_for_topic = cluster_for_topic.clone();
                                            move |this, event: &MouseDownEvent, _, cx| {
                                                this.show_topic_menu(cluster_for_topic.clone(), topic_name.clone(), event.position);
                                                cx.notify();
                                            }
                                        }))
                                }).collect()
                            })
                            .unwrap_or_default();

                        this.children(topic_divs)
                    })
            }))
    }

    /// Render tree mode (clusters with topics)
    fn render_tree_mode(
        clusters: &[crate::api::ClusterResponse],
        cluster_health: &std::collections::HashMap<String, ClusterHealth>,
        cluster_topics: &std::collections::HashMap<String, Vec<String>>,
        theme: &Theme,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .size_full()
            .children(clusters.iter().map(|cluster| {
                let health = cluster_health.get(&cluster.name);
                let healthy = health.map(|h| h.healthy).unwrap_or(false);
                let health_error = health.and_then(|h| h.error.as_ref());
                let topics_count = cluster_topics.get(&cluster.name).map(|v| v.len()).unwrap_or(0);

                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        // Cluster header
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(8.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised.opacity(0.5))
                            .cursor_pointer()
                            .child(Self::health_indicator(healthy, health_error, theme))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(cluster.name.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(format!("{} topics", topics_count))
                            )
                    )
                    .when(topics_count > 0, |this| {
                        // Show topics preview (first 5)
                        let topic_items: Vec<Div> = cluster_topics
                            .get(&cluster.name)
                            .map(|topics| {
                                topics.iter().take(5).map(|topic| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .rounded(px(3.0))
                                        .cursor_pointer()
                                        .hover(|d| d.bg(theme.surface))
                                        .child(
                                            div()
                                                .w(px(6.0))
                                                .h(px(6.0))
                                                .rounded(px(2.0))
                                                .bg(theme.text_muted.opacity(0.5))
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.text_secondary)
                                                .text_xs()
                                                .child(topic.clone())
                                        )
                                }).collect()
                            })
                            .unwrap_or_default();

                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .ml(px(16.0))
                                .gap(px(1.0))
                                .children(topic_items)
                        )
                    })
            }))
    }

    /// Render flat mode (all topics from all clusters)
    fn render_flat_mode(
        clusters: &[crate::api::ClusterResponse],
        cluster_topics: &std::collections::HashMap<String, Vec<String>>,
        theme: &Theme,
    ) -> Div {
        let topic_items: Vec<Div> = clusters
            .iter()
            .flat_map(|cluster| {
                cluster_topics
                    .get(&cluster.name)
                    .map(|topics| {
                        topics.iter().map(|topic| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child(cluster.name.clone())
                                )
                                .child(
                                    div()
                                        .text_color(theme.text)
                                        .text_sm()
                                        .child(topic.clone())
                                )
                        }).collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .size_full()
            .children(topic_items)
    }
}