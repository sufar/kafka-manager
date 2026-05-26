//! Virtual List Component
//!
//! Efficient rendering of large lists with virtual scrolling.
//! Matches vue-virtual-scroller's RecycleScroller behavior.

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

impl VirtualListConfig {
    /// Configuration for message list (24px item height like Vue3)
    pub fn for_messages(container_height: f32) -> Self {
        Self {
            item_height: 24.0,  // Vue3 uses 24px for messages
            buffer_items: 5,
            container_height,
        }
    }

    /// Configuration for topic list (40px item height)
    pub fn for_topics(container_height: f32) -> Self {
        Self {
            item_height: 40.0,
            buffer_items: 3,
            container_height,
        }
    }
}

/// Virtual list state for tracking visible range
#[derive(Debug, Clone, Default)]
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

    /// Get item index at scroll position
    pub fn item_at_position(&self, y_pos: f32, config: &VirtualListConfig) -> Option<usize> {
        let index = (y_pos / config.item_height) as usize;
        if index < self.total_items {
            Some(index)
        } else {
            None
        }
    }
}

/// Virtual list component with Entity-based scroll tracking
pub struct VirtualListWithState<T: VirtualListItem + 'static> {
    theme: Theme,
    items: Vec<T>,
    state: VirtualListState,
    config: VirtualListConfig,
    selected_index: Option<usize>,
}

impl<T: VirtualListItem + Clone> VirtualListWithState<T> {
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

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn state(&self) -> &VirtualListState {
        &self.state
    }

    pub fn config(&self) -> &VirtualListConfig {
        &self.config
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
}

/// Virtual list component (legacy, for backward compatibility)
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

    /// Render a single item at given index (for backward compatibility)
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
            .bg(if is_selected {
                theme.primary.opacity(0.15)
            } else if index % 2 == 1 {
                theme.surface_raised
            } else {
                gpui::transparent_black()
            })
            .border_l(px(3.0))
            .border_color(if is_selected {
                theme.primary
            } else {
                gpui::transparent_black()
            })
            .border_b(px(1.0))
            .border_color(theme.border.opacity(0.3))
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_xs()
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
        let total_height = state.total_height(&config);

        // Matches vue-virtual-scroller structure:
        // - Outer container with overflow_auto
        // - Inner spacer with total height
        // - Absolute positioned items within visible range
        div()
            .flex()
            .flex_col()
            .w_full()
            .h(px(config.container_height))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Inner spacer div - maintains proper scrollbar height
                div()
                    .w_full()
                    .h(px(total_height))
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
                            .bg(if is_selected {
                                theme.primary.opacity(0.15)
                            } else if index % 2 == 1 {
                                theme.surface_raised
                            } else {
                                gpui::transparent_black()
                            })
                            .border_l(px(3.0))
                            .border_color(if is_selected {
                                theme.primary
                            } else {
                                gpui::transparent_black()
                            })
                            .border_b(px(1.0))
                            .border_color(theme.border.opacity(0.3))
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                                    .text_xs()
                                    .font_weight(if is_selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                                    .child(item.id())
                            )
                    }))
            )
    }
}

/// Simple string item for testing
#[derive(Clone)]
pub struct SimpleItem {
    pub id: String,
    pub label: String,
}

impl SimpleItem {
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

/// Message item for virtual list (matches Vue's compact message format)
#[derive(Clone)]
pub struct MessageItem {
    pub id: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: i64,
}

impl MessageItem {
    pub fn offset(&self) -> i64 {
        self.offset
    }
    pub fn partition(&self) -> i32 {
        self.partition
    }
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl VirtualListItem for MessageItem {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn height(&self) -> f32 {
        24.0  // Matches Vue3's message row height
    }
}

/// Convert BufferedMessage to MessageItem
impl From<&crate::state::BufferedMessage> for MessageItem {
    fn from(msg: &crate::state::BufferedMessage) -> Self {
        Self {
            id: format!("{}-{}", msg.partition, msg.offset),
            partition: msg.partition,
            offset: msg.offset,
            key: msg.key.clone(),
            value: msg.value.clone(),
            timestamp: msg.timestamp,
        }
    }
}

/// Create MessageItems from MessageBuffer
pub fn messages_from_buffer(buffer: &crate::state::MessageBuffer<crate::state::BufferedMessage>) -> Vec<MessageItem> {
    buffer.messages().iter()
        .map(|msg| MessageItem::from(msg))
        .collect()
}

/// Create visible MessageItems from buffer with virtual scrolling
pub fn visible_messages_from_buffer(
    buffer: &crate::state::MessageBuffer<crate::state::BufferedMessage>,
    scroll_offset: f32,
    item_height: f32,
    container_height: f32,
) -> Vec<MessageItem> {
    let (start, end) = buffer.visible_range(scroll_offset, item_height, container_height);
    buffer.messages().iter()
        .skip(start)
        .take(end - start)
        .map(|msg| MessageItem::from(msg))
        .collect()
}

/// Render a message row with table columns (matches Vue3's message table layout)
pub fn render_message_row(
    theme: &Theme,
    msg: &MessageItem,
    index: usize,
    is_selected: bool,
    column_widths: &MessageColumnWidths,
) -> Div {
    let y_offset = index as f32 * 24.0;

    div()
        .flex()
        .items_center()
        .px(px(8.0))
        .h(px(24.0))  // 24px height like Vue3
        .bg(if is_selected {
            theme.primary.opacity(0.15)
        } else if index % 2 == 1 {
            theme.surface_raised
        } else {
            gpui::transparent_black()
        })
        .border_l(px(3.0))
        .border_color(if is_selected {
            theme.primary
        } else {
            gpui::transparent_black()
        })
        .border_b(px(1.0))
        .border_color(theme.border.opacity(0.3))
        .cursor_pointer()
        // Partition column (badge style like Vue3)
        .child(
            div()
                .w(px(column_widths.partition))
                .px(px(4.0))
                .py(px(1.0))
                .rounded(px(2.0))
                .bg(theme.surface_raised)
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(format!("P{}", msg.partition))
                )
        )
        // Offset column (font-mono like Vue3)
        .child(
            div()
                .w(px(column_widths.offset))
                .text_color(theme.text_muted)
                .text_xs()
                .font_family("monospace")
                .child(msg.offset.to_string())
        )
        // Timestamp column
        .child(
            div()
                .w(px(column_widths.timestamp))
                .text_color(theme.text_secondary)
                .text_xs()
                .child(format_timestamp(msg.timestamp))
        )
        // Key column (truncate)
        .child(
            div()
                .w(px(column_widths.key))
                .text_color(theme.text_secondary)
                .text_xs()
                .font_family("monospace")
                .truncate()
                .child(msg.key.clone().unwrap_or_else(|| "-".to_string()))
        )
        // Value column (flex_1, truncate)
        .child(
            div()
                .flex_1()
                .pr(px(8.0))
                .text_color(theme.text)
                .text_xs()
                .font_family("monospace")
                .truncate()
                .child(msg.value.clone())
        )
}

/// Message table column widths (matches Vue's columnWidths)
#[derive(Debug, Clone, Default)]
pub struct MessageColumnWidths {
    pub partition: f32,
    pub offset: f32,
    pub timestamp: f32,
    pub key: f32,
    pub value: f32,  // flex_1, minimum width
    pub actions: f32,
}

impl MessageColumnWidths {
    pub fn defaults() -> Self {
        Self {
            partition: 48.0,   // Matches Vue: partition: 48
            offset: 64.0,      // Matches Vue: offset: 64
            timestamp: 112.0,  // Matches Vue: timestamp: 112
            key: 80.0,         // Matches Vue: key: 80
            value: 200.0,      // Matches Vue: value: 200 (flex_1)
            actions: 40.0,     // Matches Vue: actions: 40
        }
    }
}

/// Format timestamp to human readable (matches Vue's formatTime)
fn format_timestamp(ts_ms: i64) -> String {
    use chrono::{Utc, TimeZone};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts_ms.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_list_config_defaults() {
        let config = VirtualListConfig::default();
        assert_eq!(config.item_height, 32.0);
        assert_eq!(config.buffer_items, 5);
    }

    #[test]
    fn test_message_config() {
        let config = VirtualListConfig::for_messages(400.0);
        assert_eq!(config.item_height, 24.0);  // Vue3 uses 24px
    }

    #[test]
    fn test_visible_range() {
        let config = VirtualListConfig::for_messages(400.0);
        let state = VirtualListState::new(1000, &config);

        // With 400px height and 24px items, visible count ~17 + 10 buffer = 27
        assert!(state.visible_end > state.visible_start);
        assert!(state.visible_end <= 1000);
    }

    #[test]
    fn test_scroll_update() {
        let config = VirtualListConfig::for_messages(400.0);
        let mut state = VirtualListState::new(1000, &config);

        // Scroll to item 100
        state.update(100.0 * 24.0, &config);

        // Should start before 100 (buffer items)
        assert!(state.visible_start < 100);
    }

    #[test]
    fn test_message_item_height() {
        let item = MessageItem {
            id: "test".to_string(),
            partition: 0,
            offset: 12345,
            key: None,
            value: "test".to_string(),
            timestamp: 0,
        };
        assert_eq!(item.height(), 24.0);
        assert_eq!(item.partition(), 0);
    }
}