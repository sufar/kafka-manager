//! Navigator Search Component
//!
//! Search bar for navigation tree, supporting search across clusters and topics.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Item type
    pub item_type: SearchItemType,
    /// Item name/label
    pub name: String,
    /// Parent cluster name (for topics)
    pub parent: Option<String>,
    /// Match score (higher = better match)
    pub score: f32,
}

/// Search item type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchItemType {
    Cluster,
    Topic,
    Partition,
}

impl SearchItemType {
    /// Get display icon color
    pub fn icon_color(&self, theme: &Theme) -> Hsla {
        match self {
            SearchItemType::Cluster => theme.primary,
            SearchItemType::Topic => theme.success,
            SearchItemType::Partition => theme.text_muted,
        }
    }

    /// Get display label
    pub fn label(&self, translations: &Translations) -> String {
        match self {
            SearchItemType::Cluster => translations.clusters.title.clone(),
            SearchItemType::Topic => translations.topics.title.clone(),
            SearchItemType::Partition => translations.messages.partition.clone(),
        }
    }
}

/// Navigator search component
pub struct NavigatorSearch {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Search query
    query: String,
    /// Is search focused
    is_focused: bool,
    /// Search results
    results: Vec<SearchResult>,
    /// Show results dropdown
    show_results: bool,
    /// Selected result index
    selected_result: Option<usize>,
    /// Recent searches
    recent_searches: Vec<String>,
}

impl NavigatorSearch {
    /// Create new navigator search
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            query: String::new(),
            is_focused: false,
            results: Vec::new(),
            show_results: false,
            selected_result: None,
            recent_searches: vec![
                "orders".to_string(),
                "Production".to_string(),
                "payments".to_string(),
            ],
        }
    }

    /// Set search query
    pub fn set_query(&mut self, query: String) {
        let is_empty = query.is_empty();
        self.query = query;
        if is_empty {
            self.show_results = false;
            self.results.clear();
        } else {
            self.show_results = true;
            self.update_results();
        }
    }

    /// Get current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set focused state
    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    /// Is search active (has query)
    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }

    /// Get selected result
    pub fn selected(&self) -> Option<&SearchResult> {
        self.selected_result.map(|idx| &self.results[idx])
    }

    /// Clear search
    pub fn clear(&mut self) {
        self.query.clear();
        self.results.clear();
        self.show_results = false;
        self.selected_result = None;
    }

    /// Add to recent searches
    pub fn add_recent(&mut self, query: String) {
        // Remove if already exists
        self.recent_searches.retain(|s| s != &query);
        // Add to front
        self.recent_searches.insert(0, query);
        // Keep max 10 recent searches
        if self.recent_searches.len() > 10 {
            self.recent_searches.pop();
        }
    }

    /// Update search results based on query
    fn update_results(&mut self) {
        // Mock search results
        let query_lower = self.query.to_lowercase();

        let mut results = Vec::new();

        // Mock clusters
        let clusters = ["Production", "Development", "Staging"];
        for cluster in clusters {
            if cluster.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    item_type: SearchItemType::Cluster,
                    name: cluster.to_string(),
                    parent: None,
                    score: if cluster.to_lowercase().starts_with(&query_lower) { 1.0 } else { 0.5 },
                });
            }
        }

        // Mock topics
        let topics = [
            ("orders", "Production"),
            ("payments", "Production"),
            ("test-topic", "Development"),
            ("user-events", "Production"),
            ("notifications", "Staging"),
        ];
        for (topic, cluster) in topics {
            if topic.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    item_type: SearchItemType::Topic,
                    name: topic.to_string(),
                    parent: Some(cluster.to_string()),
                    score: if topic.to_lowercase().starts_with(&query_lower) { 1.0 } else { 0.5 },
                });
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        self.results = results;
    }

    /// Render search input
    fn render_input(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(8.0))
            .bg(if self.is_focused { theme.surface_raised } else { theme.surface })
            .border(px(1.0))
            .border_color(if self.is_focused { theme.border_focused } else { theme.border })
            .child(
                // Search icon placeholder
                div()
                    .w(px(14.0))
                    .h(px(14.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted)
            )
            .child(
                // Search text or placeholder
                div()
                    .flex_1()
                    .text_color(if self.query.is_empty() {
                        theme.text_muted
                    } else {
                        theme.text
                    })
                    .text_xs()
                    .child(if self.query.is_empty() {
                        t.layout.search_placeholder.clone()
                    } else {
                        self.query.clone()
                    })
            )
            .when(!self.query.is_empty(), |this| {
                this.child(
                    // Clear button
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .rounded(px(4.0))
                        .bg(theme.surface_raised)
                        .cursor_pointer()
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("×")
                        )
                )
            })
    }

    /// Render search result item
    fn render_result_item(&self, result: &SearchResult, is_selected: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(if is_selected { theme.primary.opacity(0.15) } else { gpui::transparent_black() })
            .cursor_pointer()
            .child(
                // Type icon
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(3.0))
                    .bg(result.item_type.icon_color(theme))
            )
            .child(
                // Name
                div()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_sm()
                    .child(result.name.clone())
            )
            .when_some(result.parent.clone(), |this, parent| {
                this.child(
                    // Parent info
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(format!("({})", parent))
                )
            })
            .child(
                // Type label
                div()
                    .ml(px(8.0))
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded(px(3.0))
                    .bg(theme.surface)
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(result.item_type.label(&self.translations))
                    )
            )
    }

    /// Render search results dropdown
    fn render_results_dropdown(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .mt(px(4.0))
            .p(px(8.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .when(self.results.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child("未找到结果")
                )
            })
            .when(!self.results.is_empty(), |this| {
                this.children(self.results.iter().enumerate().map(|(idx, result)| {
                    let is_selected = self.selected_result == Some(idx);
                    self.render_result_item(result, is_selected)
                }))
            })
    }

    /// Render recent searches
    fn render_recent_searches(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .mt(px(4.0))
            .p(px(8.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child("最近搜索")
            )
            .children(self.recent_searches.iter().take(5).map(|search| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(6.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface_raised))
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(2.0))
                            .bg(theme.text_muted.opacity(0.5))
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(search.clone())
                    )
            }))
    }
}

impl IntoElement for NavigatorSearch {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .w_full()
            .child(self.render_input())
            .when(self.show_results && !self.query.is_empty(), |this| {
                this.child(self.render_results_dropdown())
            })
            .when(!self.show_results && self.query.is_empty() && !self.recent_searches.is_empty() && self.is_focused, |this| {
                this.child(self.render_recent_searches())
            })
    }
}

impl Clone for NavigatorSearch {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            query: self.query.clone(),
            is_focused: self.is_focused,
            results: self.results.clone(),
            show_results: self.show_results,
            selected_result: self.selected_result,
            recent_searches: self.recent_searches.clone(),
        }
    }
}