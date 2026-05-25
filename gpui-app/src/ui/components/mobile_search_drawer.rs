//! Mobile Search Drawer Component
//!
//! Responsive search drawer for mobile layout with slide-in animation.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{NavigatorSearch, SearchResult, SearchItemType};

/// Mobile Search Drawer - Responsive search overlay
pub struct MobileSearchDrawer {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Is drawer open
    is_open: bool,
    /// Search component
    search: NavigatorSearch,
    /// Recent searches
    recent_searches: Vec<String>,
    /// Quick actions
    quick_actions: Vec<QuickAction>,
}

/// Quick action item
#[derive(Debug, Clone)]
pub struct QuickAction {
    /// Action label
    pub label: String,
    /// Action icon type
    pub icon: QuickActionIcon,
}

/// Quick action icon type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuickActionIcon {
    Cluster,
    Topic,
    Message,
    Settings,
    Favorite,
}

impl QuickActionIcon {
    /// Get icon color
    pub fn color(&self, theme: &Theme) -> Hsla {
        match self {
            QuickActionIcon::Cluster => theme.primary,
            QuickActionIcon::Topic => theme.success,
            QuickActionIcon::Message => theme.info,
            QuickActionIcon::Settings => theme.text_muted,
            QuickActionIcon::Favorite => theme.warning,
        }
    }
}

impl MobileSearchDrawer {
    /// Create new mobile search drawer
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            is_open: false,
            search: NavigatorSearch::new(theme.clone(), Arc::new(Translations::default())),
            recent_searches: vec![
                "orders".to_string(),
                "Production".to_string(),
                "payments".to_string(),
            ],
            quick_actions: vec![
                QuickAction { label: "添加集群".to_string(), icon: QuickActionIcon::Cluster },
                QuickAction { label: "创建Topic".to_string(), icon: QuickActionIcon::Topic },
                QuickAction { label: "发送消息".to_string(), icon: QuickActionIcon::Message },
                QuickAction { label: "收藏列表".to_string(), icon: QuickActionIcon::Favorite },
                QuickAction { label: "设置".to_string(), icon: QuickActionIcon::Settings },
            ],
        }
    }

    /// Open drawer
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Close drawer
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Toggle drawer
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Is drawer open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Set search query
    pub fn set_search(&mut self, query: String) {
        self.search.set_query(query);
    }

    /// Render overlay backdrop
    fn render_backdrop(&self) -> Div {
        let theme = &self.theme;

        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(px(2000.0))
            .h(px(2000.0))
            .bg(theme.background.opacity(0.5))
            .cursor_pointer()
    }

    /// Render drawer panel
    fn render_drawer_panel(&self) -> Div {
        let theme = &self.theme;

        div()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .w(px(300.0))
            .h(px(2000.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(16.0))
            .bg(theme.surface)
            .border_l(px(2.0))
            .border_color(theme.border)
            .child(
                // Header with close button
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb(px(12.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("搜索")
                    )
                    .child(
                        // Close button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_sm()
                                    .child("×")
                            )
                    )
            )
            .child(
                // Search input
                self.search.clone()
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
                            .child("快捷操作")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .children(self.quick_actions.iter().map(|action| {
                                self.render_quick_action(action)
                            }))
                    )
            )
            .child(
                // Recent searches section
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .pt(px(12.0))
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("最近搜索")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .children(self.recent_searches.iter().map(|search| {
                                self.render_recent_search(search)
                            }))
                    )
            )
    }

    /// Render quick action item
    fn render_quick_action(&self, action: &QuickAction) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(6.0))
            .bg(theme.surface_raised)
            .cursor_pointer()
            .child(
                // Action icon
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .bg(action.icon.color(theme).opacity(0.2))
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(2.0))
                            .bg(action.icon.color(theme))
                    )
            )
            .child(
                // Action label
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(action.label.clone())
            )
    }

    /// Render recent search item
    fn render_recent_search(&self, search: &str) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(theme.surface)
            .cursor_pointer()
            .child(
                // Clock icon placeholder
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted.opacity(0.5))
            )
            .child(
                // Search term
                div()
                    .flex_1()
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child(search.clone())
            )
            .child(
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
    }
}

impl IntoElement for MobileSearchDrawer {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .relative()
            .when(self.is_open, |this| {
                this.child(self.render_backdrop())
                    .child(self.render_drawer_panel())
            })
    }
}

impl Clone for MobileSearchDrawer {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            is_open: self.is_open,
            search: self.search.clone(),
            recent_searches: self.recent_searches.clone(),
            quick_actions: self.quick_actions.clone(),
        }
    }
}