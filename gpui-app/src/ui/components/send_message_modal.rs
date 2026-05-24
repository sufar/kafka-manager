//! Send Message Modal
//!
//! Modal for sending messages to Kafka topics.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::api::SendMessageRequest;

/// Message header
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub key: String,
    pub value: String,
}

/// Send message form state
#[derive(Clone)]
pub struct SendMessageForm {
    partition: i32,
    key: String,
    value: String,
    headers: Vec<MessageHeader>,
    show_headers: bool,
}

impl Default for SendMessageForm {
    fn default() -> Self {
        Self {
            partition: 0,
            key: String::new(),
            value: String::new(),
            headers: Vec::new(),
            show_headers: false,
        }
    }
}

impl SendMessageForm {
    /// Convert to API request
    pub fn to_request(&self, topic: String) -> SendMessageRequest {
        use serde_json::json;
        let headers_json = if self.headers.is_empty() {
            None
        } else {
            Some(json!(self.headers.iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect::<std::collections::HashMap<String, String>>()))
        };

        SendMessageRequest {
            topic,
            partition: Some(self.partition),
            key: if self.key.is_empty() { None } else { Some(self.key.clone()) },
            value: self.value.clone(),
            headers: headers_json,
        }
    }
}

/// Send message modal
#[derive(Clone)]
pub struct SendMessageModal {
    theme: Theme,
    cluster_name: String,
    topic_name: String,
    partitions: Vec<i32>,
    form: SendMessageForm,
    is_open: bool,
    sending: bool,
}

impl SendMessageModal {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            cluster_name: String::new(),
            topic_name: String::new(),
            partitions: vec![0, 1, 2, 3],
            form: SendMessageForm::default(),
            is_open: false,
            sending: false,
        }
    }

    pub fn open(&mut self, cluster_name: String, topic_name: String, partitions: Vec<i32>) {
        let first_partition = partitions.first().copied().unwrap_or(0);
        self.cluster_name = cluster_name;
        self.topic_name = topic_name;
        self.partitions = partitions;
        self.form = SendMessageForm::default();
        self.form.partition = first_partition;
        self.is_open = true;
        self.sending = false;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn toggle_headers(&mut self) {
        self.form.show_headers = !self.form.show_headers;
    }

    pub fn add_header(&mut self) {
        self.form.headers.push(MessageHeader {
            key: String::new(),
            value: String::new(),
        });
    }

    pub fn remove_header(&mut self, index: usize) {
        if index < self.form.headers.len() {
            self.form.headers.remove(index);
        }
    }

    fn render_partition_select(&self) -> Div {
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
                    .child("分区")
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .w(px(120.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(self.form.partition.to_string())
                    )
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(theme.text_muted.opacity(0.5))
                    )
            )
    }

    fn render_key_input(&self) -> Div {
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
                    .child("Key (可选)")
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
                            .text_color(if self.form.key.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_sm()
                            .child(if self.form.key.is_empty() {
                                "输入消息 Key".to_string()
                            } else {
                                self.form.key.clone()
                            })
                    )
            )
    }

    fn render_value_input(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Value *")
                    )
                    .child(
                        // Format button
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .cursor_pointer()
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .bg(theme.primary.opacity(0.5))
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .min_h(px(200.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(if self.form.value.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_sm()
                            .child(if self.form.value.is_empty() {
                                "{\n  \"id\": 1,\n  \"data\": \"example\"\n}".to_string()
                            } else {
                                self.form.value.clone()
                            })
                    )
            )
    }

    fn render_header_row(&self, header: &MessageHeader, _index: usize) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                // Key input
                div()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .w(px(150.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(if header.key.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_xs()
                            .child(if header.key.is_empty() {
                                "Header Key".to_string()
                            } else {
                                header.key.clone()
                            })
                    )
            )
            .child(
                // Value input
                div()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .flex_1()
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(if header.value.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_xs()
                            .child(if header.value.is_empty() {
                                "Header Value".to_string()
                            } else {
                                header.value.clone()
                            })
                    )
            )
            .child(
                // Remove button
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.error.opacity(0.1)))
                    .child(
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .bg(theme.error.opacity(0.5))
                    )
            )
    }
}

impl IntoElement for SendMessageModal {
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
                div()
                    .flex()
                    .flex_col()
                    .w(px(600.0))
                    .max_h(px(600.0))
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
                                                    .child("发送消息")
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .text_xs()
                                                    .child(self.topic_name.clone())
                                            )
                                    )
                            )
                            .child(
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
                                // Partition and Key row
                                div()
                                    .flex()
                                    .gap(px(16.0))
                                    .child(self.render_partition_select())
                                    .child(self.render_key_input())
                            )
                            .child(self.render_value_input())
                            .child(
                                // Headers toggle
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
                                                    .child("Headers")
                                            )
                                            .when(self.form.headers.len() > 0, |d| {
                                                d.child(
                                                    div()
                                                        .px(px(6.0))
                                                        .py(px(2.0))
                                                        .rounded(px(8.0))
                                                        .bg(theme.primary.opacity(0.2))
                                                        .child(
                                                            div()
                                                                .text_color(theme.primary)
                                                                .text_xs()
                                                                .child(self.form.headers.len().to_string())
                                                        )
                                                )
                                            })
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(if self.form.show_headers { "隐藏" } else { "显示" })
                                    )
                            )
                            .when(self.form.show_headers, |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.0))
                                        .children(self.form.headers.iter().enumerate().map(|(i, h)| {
                                            self.render_header_row(h, i)
                                        }))
                                        .child(
                                            // Add header button
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .gap(px(6.0))
                                                .px(px(12.0))
                                                .py(px(6.0))
                                                .rounded(px(4.0))
                                                .bg(theme.surface_raised)
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .w(px(12.0))
                                                        .h(px(12.0))
                                                        .bg(theme.primary.opacity(0.5))
                                                )
                                                .child(
                                                    div()
                                                        .text_color(theme.text_secondary)
                                                        .text_xs()
                                                        .child("添加 Header")
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
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.primary)
                                    .cursor_pointer()
                                    .when(self.sending, |d| d.opacity(0.5))
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_sm()
                                            .child(if self.sending { "发送中..." } else { "发送" })
                                    )
                            )
                    )
            )
    }
}