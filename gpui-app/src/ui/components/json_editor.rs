//! JSON Editor Component
//!
//! Component for displaying and editing JSON data with syntax highlighting.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// JSON display format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsonFormat {
    Raw,
    Pretty,
}

/// JSON editor/viewer
#[derive(Clone)]
pub struct JsonEditor {
    theme: Theme,
    content: String,
    format: JsonFormat,
    editable: bool,
}

impl JsonEditor {
    /// Create new JSON editor
    pub fn new(theme: Theme, content: String) -> Self {
        Self {
            theme,
            content,
            format: JsonFormat::Pretty,
            editable: false,
        }
    }

    /// Set content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    /// Set format
    pub fn set_format(&mut self, format: JsonFormat) {
        self.format = format;
    }

    /// Set editable
    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// Get formatted content
    fn formatted_content(&self) -> String {
        match self.format {
            JsonFormat::Raw => self.content.clone(),
            JsonFormat::Pretty => {
                // Try to parse and pretty-print JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&self.content) {
                    serde_json::to_string_pretty(&parsed).unwrap_or(self.content.clone())
                } else {
                    self.content.clone()
                }
            }
        }
    }

    /// Render a JSON line with syntax highlighting
    fn render_line(&self, line: &str) -> Div {
        let theme = &self.theme;

        // Simple syntax highlighting based on content
        let is_key = line.contains(':') && !line.contains('{') && !line.contains('[');
        let is_string_value = line.contains('"') && !line.starts_with('"');
        let is_number = line.chars().any(|c| c.is_ascii_digit()) && !line.contains('"');
        let is_bracket = line.contains('{') || line.contains('}') || line.contains('[') || line.contains(']');

        let text_color = if is_bracket {
            theme.text_muted
        } else if is_key {
            theme.primary
        } else if is_string_value {
            theme.success
        } else if is_number {
            theme.warning
        } else {
            theme.text
        };

        div()
            .text_color(text_color)
            .text_xs()
            .child(line.to_string())
    }
}

impl IntoElement for JsonEditor {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let content = self.formatted_content();
        let lines: Vec<&str> = content.lines().collect();

        div()
            .flex()
            .flex_col()
            .w_full()
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Header with format toggle
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .bg(theme.primary.opacity(0.2))
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("JSON")
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                // Raw format button
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(if self.format == JsonFormat::Raw {
                                        theme.primary.opacity(0.2)
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if self.format == JsonFormat::Raw {
                                                theme.primary
                                            } else {
                                                theme.text_muted
                                            })
                                            .text_xs()
                                            .child("Raw")
                                    )
                            )
                            .child(
                                // Pretty format button
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(if self.format == JsonFormat::Pretty {
                                        theme.primary.opacity(0.2)
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if self.format == JsonFormat::Pretty {
                                                theme.primary
                                            } else {
                                                theme.text_muted
                                            })
                                            .text_xs()
                                            .child("Pretty")
                                    )
                            )
                            .when(self.editable, |d| {
                                d.child(
                                    // Edit button
                                    div()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_xs()
                                                .child("编辑")
                                        )
                                )
                            })
                    )
            )
            .child(
                // Content area
                div()
                    .flex()
                    .flex_col()
                    .px(px(12.0))
                    .py(px(8.0))
                    .gap(px(2.0))
                    .max_h(px(300.0))
                    .children(lines.iter().map(|line| {
                        self.render_line(line)
                    }))
            )
    }
}

/// Utility functions for JSON handling
pub fn pretty_json(content: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
}

pub fn is_valid_json(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content).is_ok()
}