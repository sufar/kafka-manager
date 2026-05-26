//! Mobile Card View for Messages
//!
//! Provides a mobile-friendly card layout for messages (matches Vue's md:hidden view).

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::ui::components::virtual_list::{MessageItem, MessageColumnWidths};
use crate::utils::format::truncate_string;
use crate::utils::time::current_timestamp_ms;

/// Mobile card item height (matches Vue's item-size="76")
pub const MOBILE_CARD_HEIGHT: f32 = 76.0;

/// Mobile message card view component
pub struct MobileMessageCardView {
    theme: Theme,
    messages: Vec<MessageItem>,
    selected_index: Option<usize>,
    scroll_offset: f32,
    container_height: f32,
}

impl MobileMessageCardView {
    pub fn new(theme: Theme, messages: Vec<MessageItem>) -> Self {
        Self {
            theme,
            messages,
            selected_index: None,
            scroll_offset: 0.0,
            container_height: 400.0,
        }
    }

    pub fn with_container_height(mut self, height: f32) -> Self {
        self.container_height = height;
        self
    }

    pub fn set_messages(&mut self, messages: Vec<MessageItem>) {
        self.messages = messages;
    }

    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }

    /// Calculate visible range based on scroll position
    fn visible_range(&self) -> (usize, usize) {
        let buffer = 3;
        let visible_count = (self.container_height / MOBILE_CARD_HEIGHT) as usize + buffer * 2;
        let first_visible = (self.scroll_offset / MOBILE_CARD_HEIGHT) as usize;
        let start = first_visible.saturating_sub(buffer);
        let end = std::cmp::min(start + visible_count, self.messages.len());
        (start, end)
    }

    /// Render a single mobile card
    fn render_card(&self, msg: &MessageItem, index: usize) -> Div {
        let theme = &self.theme;
        let is_selected = self.selected_index == Some(index);

        // Format timestamp
        let timestamp_str = format_timestamp(msg.timestamp);

        // Truncate value to 100 chars (matches Vue)
        let truncated_value = truncate_string(&msg.value, 100);

        div()
            .flex()
            .flex_col()
            .w_full()
            .px(px(8.0))
            .py(px(8.0))
            .mb(px(4.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(if is_selected {
                theme.primary
            } else {
                theme.border
            })
            .border_l(px(2.0))
            .cursor_pointer()
            // Top row: Partition badge + Offset
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            // Partition badge (badge-ghost badge-xs)
                            .child(
                                div()
                                    .px(px(4.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(theme.surface_raised)
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(format!("P{}", msg.partition))
                                    )
                            )
                            // Offset (font-mono)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .font_family("monospace")
                                    .child(format!("#{}", msg.offset))
                            )
                    )
                    // Timestamp
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(timestamp_str)
                    )
            )
            // Key row (if exists)
            .when_some(msg.key.clone(), |this, key| {
                this.child(
                    div()
                        .h(px(16.0))
                        .mb(px(4.0))
                        .child(
                            div()
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .font_family("monospace")
                                .truncate()
                                .child(format!("Key: {}", key))
                        )
                )
            })
            // Value row (truncated to 100 chars)
            .child(
                div()
                    .text_color(theme.text)
                    .text_xs()
                    .font_family("monospace")
                    .truncate()
                    .child(truncated_value)
            )
    }
}

impl IntoElement for MobileMessageCardView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let (start, end) = self.visible_range();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h(px(self.container_height))
            .p(px(8.0))
            .bg(theme.surface)
            // Empty state
            .when(self.messages.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .child(
                            div()
                                .w(px(32.0))
                                .h(px(32.0))
                                .rounded(px(4.0))
                                .bg(theme.text_muted.opacity(0.3))
                        )
                        .child(
                            div()
                                .mt(px(8.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("No messages")
                        )
                )
            })
            // Cards
            .when(!self.messages.is_empty(), |this| {
                this.children(
                    self.messages.iter().enumerate().skip(start).take(end - start)
                        .map(|(index, msg)| self.render_card(msg, index))
                )
            })
    }
}

/// Format timestamp for display
fn format_timestamp(ts_ms: i64) -> String {
    use chrono::{Utc, TimeZone};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| ts_ms.to_string())
}

/// Render mobile message card for use in MessagesView
pub fn render_mobile_message_card(
    theme: &Theme,
    msg: &MessageItem,
    index: usize,
    is_selected: bool,
) -> Div {
    let timestamp_str = format_timestamp(msg.timestamp);
    let truncated_value = truncate_string(&msg.value, 100);

    div()
        .flex()
        .flex_col()
        .w_full()
        .px(px(8.0))
        .py(px(8.0))
        .mb(px(4.0))
        .rounded(px(6.0))
        .bg(theme.surface)
        .border(px(1.0))
        .border_color(if is_selected {
            theme.primary
        } else {
            theme.border
        })
        .border_l(px(2.0))
        .cursor_pointer()
        // Top row
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(4.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .px(px(4.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(theme.surface_raised)
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child(format!("P{}", msg.partition))
                                )
                        )
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .font_family("monospace")
                                .child(format!("#{}", msg.offset))
                        )
                )
                .child(
                    div()
                        .text_color(theme.text_secondary)
                        .text_xs()
                        .child(timestamp_str)
                )
        )
        // Key row
        .when_some(msg.key.clone(), |this, key| {
            this.child(
                div()
                    .h(px(16.0))
                    .mb(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .font_family("monospace")
                            .truncate()
                            .child(format!("Key: {}", key))
                    )
            )
        })
        // Value row
        .child(
            div()
                .text_color(theme.text)
                .text_xs()
                .font_family("monospace")
                .truncate()
                .child(truncated_value)
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_card_height() {
        assert_eq!(MOBILE_CARD_HEIGHT, 76.0);  // Matches Vue's item-size
    }

    #[test]
    fn test_visible_range() {
        let view = MobileMessageCardView::new(Theme::dark(), vec![]);
        assert_eq!(view.visible_range(), (0, 0));
    }
}