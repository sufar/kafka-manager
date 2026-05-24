//! Create Topic Dialog
//!
//! Dialog for creating new Kafka topics.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::api::CreateTopicRequest;

/// Topic creation form state
#[derive(Clone)]
pub struct TopicForm {
    name: String,
    num_partitions: i32,
    replication_factor: i32,
    cleanup_policy: String,
    retention_ms: String,
    retention_bytes: String,
    segment_bytes: String,
    show_advanced: bool,
}

impl Default for TopicForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            num_partitions: 3,
            replication_factor: 1,
            cleanup_policy: "delete".to_string(),
            retention_ms: String::new(),
            retention_bytes: String::new(),
            segment_bytes: String::new(),
            show_advanced: false,
        }
    }
}

impl TopicForm {
    /// Convert to API request
    pub fn to_request(&self) -> CreateTopicRequest {
        CreateTopicRequest {
            name: self.name.clone(),
            partitions: self.num_partitions,
            replication_factor: self.replication_factor,
            config: None,
        }
    }
}

/// Create topic dialog
#[derive(Clone)]
pub struct CreateTopicDialog {
    theme: Theme,
    cluster_name: String,
    form: TopicForm,
    is_open: bool,
    submitting: bool,
}

impl CreateTopicDialog {
    /// Create new dialog
    pub fn new(theme: Theme, cluster_name: String) -> Self {
        Self {
            theme,
            cluster_name,
            form: TopicForm::default(),
            is_open: false,
            submitting: false,
        }
    }

    /// Open the dialog
    pub fn open(&mut self) {
        self.form = TopicForm::default();
        self.is_open = true;
        self.submitting = false;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Check if open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Toggle advanced options
    pub fn toggle_advanced(&mut self) {
        self.form.show_advanced = !self.form.show_advanced;
    }

    /// Get cluster name
    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }

    /// Render form field
    fn render_field(&self, label: String, placeholder: String, value: String) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(label)
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(if value.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_sm()
                            .child(if value.is_empty() {
                                placeholder
                            } else {
                                value.clone()
                            })
                    )
            )
    }

    /// Render number field
    fn render_number_field(&self, label: String, value: i32) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(label)
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(value.to_string())
                    )
            )
    }

    /// Render select field
    fn render_select_field(&self, label: String, _options: Vec<String>, selected: String) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(label)
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(selected)
                    )
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(theme.text_muted.opacity(0.5))
                    )
            )
    }
}

impl IntoElement for CreateTopicDialog {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if !self.is_open {
            return div();
        }

        let theme = &self.theme;

        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.8))
            .child(
                // Dialog container
                div()
                    .flex()
                    .flex_col()
                    .w(px(480.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .p(px(16.0))
                    .child(
                        // Header
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .pb(px(12.0))
                            .border_b(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .child(
                                        // Icon placeholder
                                        div()
                                            .w(px(36.0))
                                            .h(px(36.0))
                                            .rounded(px(10.0))
                                            .bg(theme.primary.opacity(0.2))
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .child(
                                                div()
                                                    .text_color(theme.text)
                                                    .text_base()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child("创建 Topic")
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(if self.form.name.is_empty() {
                                                        "输入 Topic 名称".to_string()
                                                    } else {
                                                        self.form.name.clone()
                                                    })
                                            )
                                    )
                            )
                            .child(
                                // Close button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(28.0))
                                    .h(px(28.0))
                                    .rounded(px(14.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface_raised))
                                    .child(
                                        div()
                                            .w(px(14.0))
                                            .h(px(14.0))
                                            .bg(theme.text_muted)
                                    )
                            )
                    )
                    .child(
                        // Form content
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(16.0))
                            .py(px(16.0))
                            .child(
                                // Topic name
                                self.render_field(
                                    "Topic 名称 *".to_string(),
                                    "输入 Topic 名称".to_string(),
                                    self.form.name.clone(),
                                )
                            )
                            .child(
                                // Partitions and replication factor
                                div()
                                    .flex()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(6.0))
                                            .w(px(200.0))
                                            .child(self.render_number_field("分区数".to_string(), self.form.num_partitions))
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child("建议: 3-10")
                                            )
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(6.0))
                                            .w(px(200.0))
                                            .child(self.render_number_field("副本因子".to_string(), self.form.replication_factor))
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child("不超过 Broker 数量")
                                            )
                                    )
                            )
                            .child(
                                // Advanced options toggle
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(8.0))
                                    .border_t(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .w(px(12.0))
                                                    .h(px(12.0))
                                                    .bg(theme.text_muted.opacity(0.5))
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.text_secondary)
                                                    .text_sm()
                                                    .child("高级选项")
                                            )
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(if self.form.show_advanced { "隐藏" } else { "显示" })
                                    )
                            )
                            .when(self.form.show_advanced, |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(12.0))
                                        .child(
                                            self.render_select_field(
                                                "清理策略".to_string(),
                                                vec!["delete".to_string(), "compact".to_string(), "delete,compact".to_string()],
                                                self.form.cleanup_policy.clone(),
                                            )
                                        )
                                        .child(
                                            self.render_field(
                                                "保留时间 (ms)".to_string(),
                                                "如: 604800000".to_string(),
                                                self.form.retention_ms.clone(),
                                            )
                                        )
                                        .child(
                                            self.render_field(
                                                "保留大小 (bytes)".to_string(),
                                                "如: -1 表示无限制".to_string(),
                                                self.form.retention_bytes.clone(),
                                            )
                                        )
                                        .child(
                                            self.render_field(
                                                "段大小 (bytes)".to_string(),
                                                "如: 1073741824".to_string(),
                                                self.form.segment_bytes.clone(),
                                            )
                                        )
                                )
                            })
                    )
                    .child(
                        // Footer
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .pt(px(12.0))
                            .border_t(px(1.0))
                            .border_color(theme.border)
                            .child(
                                // Cancel button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(theme.text_secondary)
                                            .text_sm()
                                            .child("取消")
                                    )
                            )
                            .child(
                                // Create button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.primary)
                                    .cursor_pointer()
                                    .when(self.submitting, |d| d.opacity(0.5))
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_sm()
                                            .child(if self.submitting { "创建中..." } else { "创建" })
                                    )
                            )
                    )
            )
    }
}