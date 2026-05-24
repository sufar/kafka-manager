//! Message Detail Panel
//!
//! Panel showing details of a selected message with JSON syntax highlighting.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::ui::components::JsonEditor;

/// Message detail
#[derive(Debug, Clone)]
pub struct MessageDetail {
    pub offset: i64,
    pub partition: i32,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: i64,
    pub headers: Vec<(String, String)>,
}

/// Message detail panel
#[derive(Clone)]
pub struct MessageDetailPanel {
    theme: Theme,
    detail: Option<MessageDetail>,
    format_raw: bool,
    show_headers: bool,
    panel_height: f32,
}

impl MessageDetailPanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            detail: None,
            format_raw: false,
            show_headers: false,
            panel_height: 200.0,
        }
    }

    pub fn set_detail(&mut self, detail: Option<MessageDetail>) {
        self.detail = detail;
    }

    pub fn toggle_format(&mut self) {
        self.format_raw = !self.format_raw;
    }

    pub fn toggle_headers(&mut self) {
        self.show_headers = !self.show_headers;
    }

    pub fn set_height(&mut self, height: f32) {
        self.panel_height = height;
    }

    fn format_timestamp(&self, timestamp: i64) -> String {
        format!("{}ms", timestamp)
    }

    fn render_empty(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h(px(self.panel_height))
            .child(
                div()
                    .w(px(40.0))
                    .h(px(40.0))
                    .rounded(px(20.0))
                    .bg(theme.text_muted.opacity(0.2))
            )
            .child(
                div()
                    .mt(px(8.0))
                    .text_color(theme.text_muted)
                    .text_sm()
                    .child("选择消息查看详情")
            )
    }

    fn render_header_row(&self, key: &str, value: &str) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .py(px(4.0))
            .child(
                div()
                    .text_color(theme.primary)
                    .text_xs()
                    .child(key.to_string())
            )
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(":")
            )
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child(value.to_string())
            )
    }
}

impl IntoElement for MessageDetailPanel {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        let detail = match &self.detail {
            Some(d) => d,
            None => return self.render_empty(),
        };

        // Create JsonEditor for value
        let json_editor = JsonEditor::new(theme.clone(), detail.value.clone());

        div()
            .flex()
            .flex_col()
            .h(px(self.panel_height))
            .border_t(px(1.0))
            .border_color(theme.border)
            .bg(theme.surface)
            // Header bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    // Title and metadata
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("消息详情")
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface_raised)
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(format!("P{}", detail.partition))
                                            )
                                    )
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface_raised)
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(format!("Offset: {}", detail.offset))
                                            )
                                    )
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface_raised)
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(self.format_timestamp(detail.timestamp))
                                            )
                                    )
                            )
                    )
                    // Actions
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(theme.surface_raised)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child("K")
                                    )
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(theme.surface_raised)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface))
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child("V")
                                    )
                            )
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(if self.format_raw {
                                        theme.primary.opacity(0.2)
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if self.format_raw {
                                                theme.primary
                                            } else {
                                                theme.text_muted
                                            })
                                            .text_xs()
                                            .child("Raw")
                                    )
                            )
                    )
            )
            // Content area
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .px(px(12.0))
                    .py(px(8.0))
                    .gap(px(8.0))
                    // Key section (if present)
                    .when(detail.key.is_some(), |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child("Key:")
                                )
                                .child(
                                    div()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface_raised)
                                        .child(
                                            div()
                                                .text_color(theme.text)
                                                .text_xs()
                                                .child(detail.key.clone().unwrap_or_default())
                                        )
                                )
                        )
                    })
                    // Value section with JsonEditor
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(json_editor)
                    )
                    // Headers section (if present)
                    .when(detail.headers.len() > 0, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_xs()
                                                .child(format!("Headers ({})", detail.headers.len()))
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.text_muted.opacity(0.5))
                                                .text_xs()
                                                .child(if self.show_headers { "隐藏" } else { "显示" })
                                        )
                                )
                                .when(self.show_headers, |inner| {
                                    inner.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .children(
                                                detail.headers.iter().map(|(k, v)| {
                                                    self.render_header_row(k, v)
                                                })
                                            )
                                    )
                                })
                        )
                    })
            )
    }
}