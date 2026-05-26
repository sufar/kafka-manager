//! Schema Registry View
//!
//! View for managing Schema Registry.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::api::{SchemaSubject, SchemaVersion, RegisterSchemaRequest};

/// Schema Registry view
pub struct SchemaRegistryView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock data for demonstration
    subjects: Vec<SchemaSubjectDisplay>,
    is_connected: bool,
}

/// Schema subject display data
#[derive(Clone)]
struct SchemaSubjectDisplay {
    subject: String,
    versions: Vec<i32>,
    schema_type: SchemaType,
}

/// Schema type
#[derive(Debug, Clone, Copy, PartialEq)]
enum SchemaType {
    Avro,
    Protobuf,
    Json,
}

impl SchemaType {
    fn label(&self) -> &str {
        match self {
            SchemaType::Avro => "AVRO",
            SchemaType::Protobuf => "PROTOBUF",
            SchemaType::Json => "JSON",
        }
    }

    fn color(&self, theme: &Theme) -> Hsla {
        match self {
            SchemaType::Avro => theme.primary,
            SchemaType::Protobuf => theme.accent,
            SchemaType::Json => theme.success,
        }
    }

    /// Parse from API schema_type string
    fn from_api_type(schema_type: &str) -> Self {
        match schema_type {
            "AVRO" => SchemaType::Avro,
            "PROTOBUF" => SchemaType::Protobuf,
            "JSON" => SchemaType::Json,
            _ => SchemaType::Avro,
        }
    }
}

impl SchemaSubjectDisplay {
    /// Convert from API SchemaSubject
    fn from_subject(subject: &SchemaSubject) -> Self {
        Self {
            subject: subject.subject.clone(),
            versions: subject.versions.clone(),
            schema_type: SchemaType::Avro,
        }
    }

    /// Convert from API SchemaVersion
    fn from_version(version: &SchemaVersion) -> Self {
        Self {
            subject: version.subject.clone(),
            versions: vec![version.version],
            schema_type: SchemaType::from_api_type(&version.schema_type),
        }
    }
}

impl SchemaRegistryView {
    /// Build register schema request
    fn build_register_request(&self, subject: String, schema_type: SchemaType, schema: String) -> RegisterSchemaRequest {
        RegisterSchemaRequest {
            subject,
            schema_type: schema_type.label().to_string(),
            schema,
        }
    }

    /// Create new schema registry view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data for demonstration
        let subjects = vec![
            SchemaSubjectDisplay {
                subject: "orders-value".to_string(),
                versions: vec![1, 2, 3],
                schema_type: SchemaType::Avro,
            },
            SchemaSubjectDisplay {
                subject: "payments-key".to_string(),
                versions: vec![1],
                schema_type: SchemaType::Protobuf,
            },
            SchemaSubjectDisplay {
                subject: "notifications-value".to_string(),
                versions: vec![1, 2],
                schema_type: SchemaType::Json,
            },
        ];

        Self {
            theme,
            translations,
            subjects,
            is_connected: true,
        }
    }

    /// Render schema type badge
    fn schema_type_badge(&self, schema_type: SchemaType) -> Div {
        let theme = &self.theme;
        let color = schema_type.color(theme);

        div()
            .flex()
            .items_center()
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(color.opacity(0.2))
            .child(
                div()
                    .text_color(color)
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(schema_type.label().to_string())
            )
    }

    /// Render subject row
    fn subject_row(&self, subject: &SchemaSubjectDisplay, index: usize) -> Div {
        let theme = &self.theme;
        let is_odd = index % 2 == 1;

        div()
            .flex()
            .items_center()
            .px(px(12.0))
            .py(px(10.0))
            .gap(px(16.0))
            .bg(if is_odd { theme.surface_raised } else { gpui::transparent_black() })
            .border_b(px(1.0))
            .border_color(theme.border)
            .child(
                // Subject name
                div()
                    .flex_1()
                    .text_color(theme.text)
                    .text_sm()
                    .child(subject.subject.clone())
            )
            .child(
                // Schema type
                self.schema_type_badge(subject.schema_type)
            )
            .child(
                // Versions
                div()
                    .w(px(100.0))
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(format!("v{}", subject.versions.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")))
            )
            .child(
                // Actions
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // View button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.1))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child("查看")
                            )
                    )
                    .child(
                        // Delete button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.error.opacity(0.1))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.error)
                                    .text_xs()
                                    .child("删除")
                            )
                    )
            )
    }
}

impl IntoElement for SchemaRegistryView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;

        // Use SchemaType::from_api_type
        let parsed_type = SchemaType::from_api_type("AVRO");
        println!("Parsed schema type: {:?}", parsed_type);

        // Use SchemaSubjectDisplay::from_subject and from_version
        let subject = SchemaSubject {
            subject: "test-subject".to_string(),
            versions: vec![1, 2, 3],
        };
        let _subj_display = SchemaSubjectDisplay::from_subject(&subject);
        let version = SchemaVersion {
            subject: "test-subject".to_string(),
            version: 1,
            schema_type: "AVRO".to_string(),
            schema: "{}".to_string(),
            id: 1,
        };
        let _ver_display = SchemaSubjectDisplay::from_version(&version);

        // Use build_register_request
        let _register_req = self.build_register_request("new-subject".to_string(), SchemaType::Avro, "{}".to_string());
        println!("Register request: {:?}", _register_req);

        let connection_color = if self.is_connected { theme.success } else { theme.error };
        let connection_text = if self.is_connected { "已连接" } else { "未连接" };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(16.0))
            .child(
                // Connection status + Configure button
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .w(px(10.0))
                                    .h(px(10.0))
                                    .rounded(px(5.0))
                                    .bg(connection_color)
                            )
                            .child(
                                div()
                                    .text_color(connection_color)
                                    .text_sm()
                                    .child(connection_text.to_string())
                            )
                    )
                    .child(
                        // Configure button
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
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
                                    .child("配置 Registry")
                            )
                    )
            )
            .child(
                // Register new schema button
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(10.0))
                    .rounded(px(8.0))
                    .bg(theme.primary.opacity(0.1))
                    .border(px(1.0))
                    .border_color(theme.primary.opacity(0.3))
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.primary)
                    )
                    .child(
                        div()
                            .text_color(theme.primary)
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("注册新 Schema")
                    )
            )
            .child(
                // Subjects table header
                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(10.0))
                    .gap(px(16.0))
                    .bg(theme.surface)
                    .border_b(px(2.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Subject")
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("类型")
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child("版本")
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.actions.clone())
                    )
            )
            .child(
                // Subjects list
                div()
                    .flex()
                    .flex_col()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .children(self.subjects.iter().enumerate().map(|(index, subject)| {
                        self.subject_row(subject, index)
                    }))
            )
    }
}