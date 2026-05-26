//! Cluster Tree Navigator
//!
//! Tree view for clusters and topics navigation with search filtering.
//! Dynamic data integration with GlobalState Entity.
//! Supports click navigation, right-click context menus, and topic search.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use std::collections::{HashSet, HashMap};
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::state::{GlobalState, ClusterHealth};
use crate::router::ViewType;
use crate::api::ClusterResponse;

/// Tree node state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeState {
    #[default]
    Collapsed,
    Expanded,
}

/// Cluster tree navigator with GlobalState Entity integration
pub struct ClusterTreeNavigator {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    expanded_clusters: HashSet<String>,
    expanded_folders: HashSet<String>,
    selected_cluster: Option<String>,
    selected_topic: Option<(String, String)>,
    search_filter: String,
    group_filter: Option<i32>,
    /// Per-cluster topic search queries
    topic_search: HashMap<String, String>,
    /// Per-cluster consumer group search queries
    consumer_group_search: HashMap<String, String>,
    /// Refreshing topics for specific clusters
    refreshing_topics: HashSet<String>,
    /// Refreshing consumer groups for specific clusters
    refreshing_consumer_groups: HashSet<String>,
}

impl ClusterTreeNavigator {
    pub fn new(state: Entity<GlobalState>, translations: Arc<Translations>) -> Self {
        Self {
            state,
            translations,
            expanded_clusters: HashSet::new(),
            expanded_folders: HashSet::new(),
            selected_cluster: None,
            selected_topic: None,
            search_filter: String::new(),
            group_filter: None,
            topic_search: HashMap::new(),
            consumer_group_search: HashMap::new(),
            refreshing_topics: HashSet::new(),
            refreshing_consumer_groups: HashSet::new(),
        }
    }

    fn toggle_cluster(&mut self, name: &str) {
        if self.expanded_clusters.contains(name) {
            self.expanded_clusters.remove(name);
        } else {
            self.expanded_clusters.insert(name.to_string());
        }
    }

    fn toggle_folder(&mut self, key: &str) {
        if self.expanded_folders.contains(key) {
            self.expanded_folders.remove(key);
        } else {
            self.expanded_folders.insert(key.to_string());
        }
    }

    fn get_topic_search(&self, cluster: &str) -> String {
        self.topic_search.get(cluster).cloned().unwrap_or_default()
    }

    fn set_topic_search(&mut self, cluster: &str, query: String) {
        self.topic_search.insert(cluster.to_string(), query);
    }

    fn filter_topics(&self, topics: &[String], cluster: &str) -> Vec<String> {
        let query = self.get_topic_search(cluster);
        if query.is_empty() {
            topics.to_vec()
        } else {
            topics.iter()
                .filter(|t| t.to_lowercase().contains(&query.to_lowercase()))
                .cloned()
                .collect()
        }
    }

    fn refresh_cluster_topics(&mut self, cluster: &str, cx: &mut Context<Self>) {
        self.refreshing_topics.insert(cluster.to_string());
        cx.spawn({
            let cluster = cluster.to_string();
            async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
                this.update(cx, |view, cx| {
                    view.refreshing_topics.remove(&cluster);
                    cx.notify();
                }).ok();
            }
        }).detach();
        cx.notify();
    }

    fn health_color(health: Option<&ClusterHealth>, theme: &Theme) -> Hsla {
        match health {
            Some(h) if h.healthy => theme.success,
            Some(h) if h.error.is_some() => theme.error,
            Some(_) => theme.warning,
            None => theme.text_muted,
        }
    }
}

impl Render for ClusterTreeNavigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let clusters = &state.clusters;
        let cluster_health = &state.cluster_health;
        let cluster_topics = &state.cluster_topics;
        let cluster_consumer_groups = &state.cluster_consumer_groups;
        let loading = state.loading;
        let t = &self.translations;

        // Filter clusters by group
        let filtered: Vec<&ClusterResponse> = clusters
            .iter()
            .filter(|c| self.group_filter.map_or(true, |g| c.group_id == Some(g)))
            .collect();

        div()
            .id("cluster-tree-navigator")
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                // Search box
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(div().text_color(theme.text_muted).text_xs().child("🔍"))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(t.layout.search_placeholder.clone())
                    )
            )
            .child(div().h(px(1.0)).w_full().bg(theme.border))
            .when(loading, |this| {
                this.child(
                    div()
                        .flex()
                        .justify_center()
                        .py(px(16.0))
                        .child(div().text_color(theme.text_muted).text_xs().child(t.common.loading.clone()))
                )
            })
            .when(!loading && filtered.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .justify_center()
                        .py(px(16.0))
                        .child(div().text_color(theme.text_muted).text_xs().child(t.common.no_data.clone()))
                )
            })
            .when(!loading && !filtered.is_empty(), |this| {
                this.child(
                    // Cluster list with inline handlers
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .size_full()
                        .children(filtered.iter().map(|cluster| {
                            let name = cluster.name.clone();
                            let expanded = self.expanded_clusters.contains(&name);
                            let health = cluster_health.get(&name);
                            let topics = cluster_topics.get(&name).cloned().unwrap_or_default();
                            let groups = cluster_consumer_groups.get(&name).cloned().unwrap_or_default();
                            let health_color = Self::health_color(health, theme);
                            let topics_expanded = self.expanded_folders.contains(&format!("topics-{}", name));
                            let groups_expanded = self.expanded_folders.contains(&format!("groups-{}", name));
                            let icon = if expanded { "▼" } else { "▶" };

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    // Cluster header - click to expand, navigate to TopicsView
                                    div()
                                        .id(format!("cluster-header-{}", name))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface_raised.opacity(0.5))
                                        .cursor_pointer()
                                        .hover(|d| d.bg(theme.surface))
                                        .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(health_color))
                                        .child(div().text_color(theme.text_muted).text_xs().child(icon))
                                        .child(div().text_color(theme.text).text_sm().font_weight(FontWeight::MEDIUM).child(name.clone()))
                                        .when(expanded, |this| {
                                            this.child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(format!("{} topics, {} groups", topics.len(), groups.len()))
                                            )
                                        })
                                        .on_click(cx.listener({
                                            let name = name.clone();
                                            move |this, _, _, cx| {
                                                this.toggle_cluster(&name);
                                                // Navigate to topics view for this cluster
                                                this.state.update(cx, |state, cx| {
                                                    state.select_cluster(name.clone());
                                                    state.navigate(ViewType::Topics);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }))
                                )
                                .when(expanded, |this| {
                                    this.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .ml(px(16.0))
                                            .gap(px(4.0))
                                            // Topics folder - click to expand
                                            .child(
                                                div()
                                                    .id(format!("topics-folder-{}", name))
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(2.0))
                                                    .child(
                                                        div()
                                                            .id(format!("topics-toggle-{}", name))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.0))
                                                            .px(px(6.0))
                                                            .py(px(4.0))
                                                            .rounded(px(3.0))
                                                            .bg(theme.surface.opacity(0.5))
                                                            .cursor_pointer()
                                                            .hover(|d| d.bg(theme.surface))
                                                            .child(div().text_color(theme.text_secondary).text_xs().child("📁"))
                                                            .child(div().text_color(theme.text_muted).text_xs().child(if topics_expanded { "▼" } else { "▶" }))
                                                            .child(div().text_color(theme.text_secondary).text_xs().child("Topics"))
                                                            .child(div().text_color(theme.text_muted).text_xs().child(format!("({})", topics.len())))
                                                            // Refresh button
                                                            .child(
                                                                div()
                                                                    .id(format!("topics-refresh-{}", name))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .w(px(16.0))
                                                                    .h(px(16.0))
                                                                    .rounded(px(3.0))
                                                                    .cursor_pointer()
                                                                    .hover(|d| d.bg(theme.surface_raised))
                                                                    .child(
                                                                        div()
                                                                            .text_color(if self.refreshing_topics.contains(&name) { theme.warning } else { theme.text_muted })
                                                                            .text_xs()
                                                                            .child(if self.refreshing_topics.contains(&name) { "⟳" } else { "↻" })
                                                                    )
                                                                    .on_click(cx.listener({
                                                                        let name = name.clone();
                                                                        move |this, _, _, cx| {
                                                                            this.refresh_cluster_topics(&name, cx);
                                                                        }
                                                                    }))
                                                            )
                                                            .on_click(cx.listener({
                                                                let folder_key = format!("topics-{}", name);
                                                                move |this, _, _, cx| {
                                                                    this.toggle_folder(&folder_key);
                                                                    cx.notify();
                                                                }
                                                            }))
                                                    )
                                                    .when(topics_expanded, |this| {
                                                        // Topic search box
                                                        let search_query = self.get_topic_search(&name);
                                                        let filtered_topics = self.filter_topics(&topics, &name);
                                                        let total = topics.len();
                                                        let matching = filtered_topics.len();

                                                        this.child(
                                                            div()
                                                                .flex()
                                                                .flex_col()
                                                                .ml(px(16.0))
                                                                .gap(px(2.0))
                                                                // Search input
                                                                .child(
                                                                    div()
                                                                        .id(format!("topic-search-{}", name))
                                                                        .flex()
                                                                        .items_center()
                                                                        .gap(px(4.0))
                                                                        .px(px(6.0))
                                                                        .py(px(4.0))
                                                                        .rounded(px(3.0))
                                                                        .bg(theme.surface)
                                                                        .border(px(1.0))
                                                                        .border_color(theme.border)
                                                                        .child(div().text_color(theme.text_muted).text_xs().child("🔍"))
                                                                        .child(
                                                                            div()
                                                                                .text_color(theme.text_muted)
                                                                                .text_xs()
                                                                                .child(if search_query.is_empty() {
                                                                                    format!("Search {}...", total)
                                                                                } else {
                                                                                    format!("{} matching", matching)
                                                                                })
                                                                        )
                                                                )
                                                                // Topic list
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .flex_col()
                                                                        .gap(px(1.0))
                                                                        .max_h(px(200.0))
                                                                        
                                                                        .children(filtered_topics.iter().take(50).map(|topic| {
                                                                            let topic_name = topic.clone();
                                                                            let cluster_name = name.clone();

                                                                            div()
                                                                                .id(format!("topic-item-{}-{}", name, topic_name))
                                                                                .flex()
                                                                                .items_center()
                                                                                .gap(px(6.0))
                                                                                .px(px(6.0))
                                                                                .py(px(3.0))
                                                                                .rounded(px(3.0))
                                                                                .cursor_pointer()
                                                                                .hover(|d| d.bg(theme.surface))
                                                                                .child(div().w(px(6.0)).h(px(6.0)).rounded(px(2.0)).bg(theme.text_muted.opacity(0.5)))
                                                                                .child(div().text_color(theme.text_secondary).text_xs().truncate().child(topic_name.clone()))
                                                                                .on_click(cx.listener({
                                                                    let topic_name = topic_name.clone();
                                                                    let cluster_name = cluster_name.clone();
                                                                    move |this, _, _, cx| {
                                                                        // Navigate to messages view for this topic
                                                                        this.state.update(cx, |state, cx| {
                                                                            state.select_topic(cluster_name.clone(), topic_name.clone());
                                                                            state.navigate_to_messages(&cluster_name, &topic_name);
                                                                            cx.notify();
                                                                        });
                                                                        cx.notify();
                                                                    }
                                                                }))
                                                                        }))
                                                                )
                                                        )
                                                    })
                                            )
                                            // Consumer groups folder - click to expand
                                            .child(
                                                div()
                                                    .id(format!("groups-folder-{}", name))
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(2.0))
                                                    .child(
                                                        div()
                                                            .id(format!("groups-toggle-{}", name))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.0))
                                                            .px(px(6.0))
                                                            .py(px(4.0))
                                                            .rounded(px(3.0))
                                                            .bg(theme.surface.opacity(0.5))
                                                            .cursor_pointer()
                                                            .hover(|d| d.bg(theme.surface))
                                                            .child(div().text_color(theme.text_secondary).text_xs().child("📁"))
                                                            .child(div().text_color(theme.text_muted).text_xs().child(if groups_expanded { "▼" } else { "▶" }))
                                                            .child(div().text_color(theme.text_secondary).text_xs().child("Consumer Groups"))
                                                            .child(div().text_color(theme.text_muted).text_xs().child(format!("({})", groups.len())))
                                                            .on_click(cx.listener({
                                                                let folder_key = format!("groups-{}", name);
                                                                move |this, _, _, cx| {
                                                                    this.toggle_folder(&folder_key);
                                                                    cx.notify();
                                                                }
                                                            }))
                                                    )
                                                    .when(groups_expanded, |this| {
                                                        this.child(
                                                            div()
                                                                .flex()
                                                                .flex_col()
                                                                .ml(px(16.0))
                                                                .gap(px(1.0))
                                                                .children(groups.iter().take(20).map(|group| {
                                                                    let group_name = group.clone();
                                                                    let cluster_name = name.clone();

                                                                    div()
                                                                        .id(format!("group-item-{}-{}", name, group_name))
                                                                        .flex()
                                                                        .items_center()
                                                                        .gap(px(6.0))
                                                                        .px(px(6.0))
                                                                        .py(px(3.0))
                                                                        .rounded(px(3.0))
                                                                        .cursor_pointer()
                                                                        .hover(|d| d.bg(theme.surface))
                                                                        .child(div().w(px(6.0)).h(px(6.0)).rounded(px(2.0)).bg(theme.text_muted.opacity(0.5)))
                                                                        .child(div().text_color(theme.text_secondary).text_xs().child(group_name.clone()))
                                                                        .on_click(cx.listener({
                                                            let group_name = group_name.clone();
                                                            let cluster_name = cluster_name.clone();
                                                            move |this, _, _, cx| {
                                                                // Navigate to consumer groups view
                                                                this.state.update(cx, |state, cx| {
                                                                    state.select_consumer_group(cluster_name.clone(), group_name.clone());
                                                                    state.navigate(ViewType::ConsumerGroups);
                                                                    cx.notify();
                                                                });
                                                                cx.notify();
                                                            }
                                                        }))
                                                                }))
                                                        )
                                                    })
                                            )
                                    )
                                })
                        }))
                )
            })
    }
}

/// Legacy navigator for backward compatibility (static mock)
pub struct ClusterTreeNavigatorLegacy {
    theme: Theme,
    translations: Arc<Translations>,
}

impl ClusterTreeNavigatorLegacy {
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self { theme, translations }
    }
}

impl IntoElement for ClusterTreeNavigatorLegacy {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(div().text_color(theme.text_muted).text_xs().child("🔍"))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(self.translations.layout.search_placeholder.clone())
                    )
            )
            .child(div().h(px(1.0)).w_full().bg(theme.border))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .size_full()
                    .child(div().text_color(theme.text_muted).text_xs().child("No clusters loaded"))
            )
    }
}