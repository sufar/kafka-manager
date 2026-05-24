//! Virtual List Component
//!
//! Efficient rendering of large lists with virtual scrolling.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Virtual list item trait
pub trait VirtualListItem: Clone {
    fn id(&self) -> String;
    fn height(&self) -> f32;
}

/// Virtual list configuration
#[derive(Debug, Clone)]
pub struct VirtualListConfig {
    pub item_height: f32,
    pub buffer_items: usize,
    pub container_height: f32,
}

impl Default for VirtualListConfig {
    fn default() -> Self {
        Self {
            item_height: 32.0,
            buffer_items: 5,
            container_height: 400.0,
        }
    }
}

/// Virtual list state for tracking visible range
#[derive(Debug, Clone)]
pub struct VirtualListState {
    pub scroll_offset: f32,
    pub visible_start: usize,
    pub visible_end: usize,
    pub total_items: usize,
}

impl VirtualListState {
    pub fn new(total_items: usize, config: &VirtualListConfig) -> Self {
        let visible_count = (config.container_height / config.item_height) as usize + config.buffer_items * 2;
        Self {
            scroll_offset: 0.0,
            visible_start: 0,
            visible_end: std::cmp::min(visible_count, total_items),
            total_items,
        }
    }

    pub fn update(&mut self, scroll_offset: f32, config: &VirtualListConfig) {
        self.scroll_offset = scroll_offset;

        // Calculate visible range based on scroll position
        let first_visible = (scroll_offset / config.item_height) as usize;
        self.visible_start = std::cmp::max(0, first_visible.saturating_sub(config.buffer_items));

        let visible_count = (config.container_height / config.item_height) as usize + config.buffer_items * 2;
        self.visible_end = std::cmp::min(self.visible_start + visible_count, self.total_items);
    }

    pub fn visible_range(&self) -> (usize, usize) {
        (self.visible_start, self.visible_end)
    }

    pub fn total_height(&self, config: &VirtualListConfig) -> f32 {
        self.total_items as f32 * config.item_height
    }
}

/// Virtual list component
#[derive(Clone)]
pub struct VirtualList<T: VirtualListItem> {
    theme: Theme,
    items: Vec<T>,
    state: VirtualListState,
    config: VirtualListConfig,
    selected_index: Option<usize>,
}

impl<T: VirtualListItem> VirtualList<T> {
    pub fn new(theme: Theme, items: Vec<T>, config: VirtualListConfig) -> Self {
        let state = VirtualListState::new(items.len(), &config);
        Self {
            theme,
            items,
            state,
            config,
            selected_index: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        let count = items.len();
        self.items = items;
        self.state = VirtualListState::new(count, &self.config);
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.state.update(offset, &self.config);
    }

    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn visible_items(&self) -> Vec<(usize, &T)> {
        let (start, end) = self.state.visible_range();
        self.items.iter()
            .enumerate()
            .skip(start)
            .take(end - start)
            .collect()
    }

    pub fn render_item(&self, item: &T, index: usize) -> Div {
        let theme = &self.theme;
        let is_selected = self.selected_index == Some(index);
        let y_offset = index as f32 * self.config.item_height;

        div()
            .absolute()
            .top(px(y_offset))
            .left(px(0.0))
            .right(px(0.0))
            .h(px(self.config.item_height))
            .flex()
            .items_center()
            .px(px(12.0))
            .rounded(px(4.0))
            .bg(if is_selected {
                theme.primary.opacity(0.2)
            } else {
                gpui::transparent_black()
            })
            .border_l(px(2.0))
            .border_color(if is_selected {
                theme.primary
            } else {
                gpui::transparent_black()
            })
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_sm()
                    .child(item.id())
            )
    }
}

impl<T: VirtualListItem + Clone> IntoElement for VirtualList<T> {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let state = self.state.clone();
        let config = self.config.clone();
        let items = self.items.clone();
        let selected = self.selected_index;
        let (start, end) = state.visible_range();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h(px(config.container_height))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .children(items.iter().enumerate().skip(start).take(end - start).map(|(index, item)| {
                let is_selected = selected == Some(index);
                let y_offset = index as f32 * config.item_height;

                div()
                    .absolute()
                    .top(px(y_offset))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(px(config.item_height))
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .rounded(px(4.0))
                    .bg(if is_selected {
                        theme.primary.opacity(0.2)
                    } else {
                        gpui::transparent_black()
                    })
                    .border_l(px(2.0))
                    .border_color(if is_selected {
                        theme.primary
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(if is_selected { theme.text } else { theme.text_secondary })
                            .text_sm()
                            .child(item.id())
                    )
            }))
    }
}

/// Simple string item for testing
#[derive(Clone)]
pub struct SimpleItem {
    pub id: String,
    pub label: String,
}

impl SimpleItem {
    /// Get label
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl VirtualListItem for SimpleItem {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn height(&self) -> f32 {
        32.0
    }
}

/// Message item for virtual list
#[derive(Clone)]
pub struct MessageItem {
    pub id: String,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: i64,
}

impl MessageItem {
    /// Get offset
    pub fn offset(&self) -> i64 {
        self.offset
    }
    /// Get key
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    /// Get value
    pub fn value(&self) -> &str {
        &self.value
    }
    /// Get timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl VirtualListItem for MessageItem {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn height(&self) -> f32 {
        40.0 // Slightly taller for message content
    }
}