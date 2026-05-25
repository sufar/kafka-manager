//! Message Query Tool Component
//!
//! Toolbar for configuring Kafka message query parameters.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::Input;

/// Query mode for fetching messages
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum QueryMode {
    #[default]
    Newest,
    Oldest,
}

impl QueryMode {
    /// Get display string for this mode
    pub fn display(&self, translations: &Translations) -> String {
        match self {
            QueryMode::Newest => translations.messages.newest.clone(),
            QueryMode::Oldest => translations.messages.oldest.clone(),
        }
    }

    /// Convert to API query mode
    pub fn to_api_mode(&self) -> crate::api::QueryMode {
        match self {
            QueryMode::Newest => crate::api::QueryMode::Newest,
            QueryMode::Oldest => crate::api::QueryMode::Oldest,
        }
    }
}

/// Time preset for quick time range selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimePreset {
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    OneDay,
}

impl TimePreset {
    /// Get display string for this preset
    pub fn display(&self) -> String {
        match self {
            TimePreset::FiveMinutes => "5分钟".to_string(),
            TimePreset::FifteenMinutes => "15分钟".to_string(),
            TimePreset::ThirtyMinutes => "30分钟".to_string(),
            TimePreset::OneHour => "1小时".to_string(),
            TimePreset::OneDay => "1天".to_string(),
        }
    }

    /// Get milliseconds offset from now
    pub fn offset_ms(&self) -> i64 {
        match self {
            TimePreset::FiveMinutes => 5 * 60 * 1000,
            TimePreset::FifteenMinutes => 15 * 60 * 1000,
            TimePreset::ThirtyMinutes => 30 * 60 * 1000,
            TimePreset::OneHour => 60 * 60 * 1000,
            TimePreset::OneDay => 24 * 60 * 60 * 1000,
        }
    }
}

/// Streaming progress information
#[derive(Debug, Clone, Default)]
pub struct StreamingProgress {
    /// Number of messages received
    pub received: usize,
    /// Target number of messages
    pub total: usize,
    /// Is currently streaming
    pub is_streaming: bool,
}

impl StreamingProgress {
    /// Create new progress
    pub fn new(received: usize, total: usize, is_streaming: bool) -> Self {
        Self { received, total, is_streaming }
    }

    /// Get progress percentage (0.0 - 1.0)
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.received as f32 / self.total as f32
        }
    }

    /// Format progress text
    pub fn format_progress(&self) -> String {
        format!("{} / {}", self.received, self.total)
    }
}

/// Query parameters extracted from the tool state
#[derive(Debug, Clone)]
pub struct QueryParams {
    /// Selected partition (None = all partitions)
    pub partition: Option<i32>,
    /// Query mode
    pub mode: QueryMode,
    /// Max messages limit
    pub max_messages: i32,
    /// Search keyword
    pub search: Option<String>,
    /// Start time (ISO string)
    pub start_time: Option<String>,
    /// End time (ISO string)
    pub end_time: Option<String>,
}

/// Message Query Tool - Toolbar for message query configuration
pub struct MessageQueryTool {
    /// Theme for styling
    theme: Theme,
    /// Translations for i18n
    translations: Arc<Translations>,
    /// Selected partition
    selected_partition: Option<i32>,
    /// Available partitions
    partitions: Vec<i32>,
    /// Query mode
    query_mode: QueryMode,
    /// Max messages limit
    max_messages: i32,
    /// Search keyword
    search_keyword: String,
    /// Show time filters dropdown
    show_time_filters: bool,
    /// Selected time preset
    selected_time_preset: Option<TimePreset>,
    /// Start time (ISO format)
    start_time: Option<String>,
    /// End time (ISO format)
    end_time: Option<String>,
    /// Is currently querying
    is_querying: bool,
    /// Streaming progress
    streaming_progress: StreamingProgress,
    /// Search input field
    search_input: Input,
}

impl MessageQueryTool {
    /// Create new message query tool
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            selected_partition: None,
            partitions: vec![0, 1, 2], // Default mock partitions
            query_mode: QueryMode::default(),
            max_messages: 100,
            search_keyword: String::new(),
            show_time_filters: false,
            selected_time_preset: None,
            start_time: None,
            end_time: None,
            is_querying: false,
            streaming_progress: StreamingProgress::default(),
            search_input: Input::new(theme, "搜索消息".to_string()),
        }
    }

    /// Create with specific partitions
    pub fn with_partitions(theme: Theme, translations: Arc<Translations>, partitions: Vec<i32>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            selected_partition: None,
            partitions,
            query_mode: QueryMode::default(),
            max_messages: 100,
            search_keyword: String::new(),
            show_time_filters: false,
            selected_time_preset: None,
            start_time: None,
            end_time: None,
            is_querying: false,
            streaming_progress: StreamingProgress::default(),
            search_input: Input::new(theme, "搜索消息".to_string()),
        }
    }

    /// Set selected partition
    pub fn set_partition(&mut self, partition: Option<i32>) {
        self.selected_partition = partition;
    }

    /// Set partitions list
    pub fn set_partitions(&mut self, partitions: Vec<i32>) {
        self.partitions = partitions;
    }

    /// Set query mode
    pub fn set_query_mode(&mut self, mode: QueryMode) {
        self.query_mode = mode;
    }

    /// Set max messages
    pub fn set_max_messages(&mut self, max: i32) {
        self.max_messages = max;
    }

    /// Set search keyword
    pub fn set_search(&mut self, search: String) {
        self.search_keyword = search;
    }

    /// Set time preset
    pub fn set_time_preset(&mut self, preset: TimePreset) {
        self.selected_time_preset = Some(preset);
        // Calculate actual time range
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::milliseconds(preset.offset_ms());
        self.start_time = Some(start.to_rfc3339());
        self.end_time = Some(now.to_rfc3339());
    }

    /// Clear time preset
    pub fn clear_time_preset(&mut self) {
        self.selected_time_preset = None;
        self.start_time = None;
        self.end_time = None;
    }

    /// Set querying state
    pub fn set_querying(&mut self, is_querying: bool) {
        self.is_querying = is_querying;
    }

    /// Set streaming progress
    pub fn set_progress(&mut self, progress: StreamingProgress) {
        self.streaming_progress = progress;
    }

    /// Get current query parameters
    pub fn get_params(&self) -> QueryParams {
        QueryParams {
            partition: self.selected_partition,
            mode: self.query_mode,
            max_messages: self.max_messages,
            search: if self.search_keyword.is_empty() { None } else { Some(self.search_keyword.clone()) },
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
        }
    }

    /// Render partition selector
    fn render_partition_selector(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(t.messages.partition.clone())
            )
            .children(
                // "All partitions" option
                std::iter::once({
                    let is_selected = self.selected_partition.is_none();
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(if is_selected { theme.primary } else { theme.surface })
                        .border(px(1.0))
                        .border_color(if is_selected { theme.primary } else { theme.border })
                        .cursor_pointer()
                        .child(
                            div()
                                .text_color(if is_selected {
                                    Hsla::from(gpui::rgb(0xffffff))
                                } else {
                                    theme.text_secondary
                                })
                                .text_xs()
                                .child(t.messages.all_partitions.clone())
                        )
                })
            )
            .children(
                // Individual partition options
                self.partitions.iter().map(|p| {
                    let is_selected = self.selected_partition == Some(*p);
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(if is_selected { theme.primary } else { theme.surface })
                        .border(px(1.0))
                        .border_color(if is_selected { theme.primary } else { theme.border })
                        .cursor_pointer()
                        .child(
                            div()
                                .text_color(if is_selected {
                                    Hsla::from(gpui::rgb(0xffffff))
                                } else {
                                    theme.text_secondary
                                })
                                .text_xs()
                                .child(format!("{}", p))
                        )
                })
            )
    }

    /// Render query mode selector
    fn render_mode_selector(&self) -> Div {
        let theme = &self.theme;
        let modes = [QueryMode::Newest, QueryMode::Oldest];

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(
                modes.iter().map(|mode| {
                    let is_selected = self.query_mode == *mode;
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(if is_selected { theme.surface_raised } else { theme.surface })
                        .border(px(1.0))
                        .border_color(if is_selected { theme.border_focused } else { theme.border })
                        .cursor_pointer()
                        .child(
                            div()
                                .text_color(if is_selected { theme.text } else { theme.text_muted })
                                .text_xs()
                                .child(mode.display(&self.translations))
                        )
                })
            )
    }

    /// Render max messages input
    fn render_max_messages(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(t.messages.max_messages.clone())
            )
            .child(
                div()
                    .text_color(theme.text)
                    .text_xs()
                    .child(self.max_messages.to_string())
            )
    }

    /// Render search input
    fn render_search_input(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .flex_1()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Search icon placeholder
                div()
                    .w(px(14.0))
                    .h(px(14.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted)
            )
            .child(
                div()
                    .text_color(if self.search_keyword.is_empty() {
                        theme.text_muted
                    } else {
                        theme.text
                    })
                    .text_xs()
                    .child(if self.search_keyword.is_empty() {
                        t.messages.search_value.clone()
                    } else {
                        self.search_keyword.clone()
                    })
            )
    }

    /// Render time preset selector
    fn render_time_preset(&self) -> Div {
        let theme = &self.theme;
        let presets = [
            TimePreset::FiveMinutes,
            TimePreset::FifteenMinutes,
            TimePreset::ThirtyMinutes,
            TimePreset::OneHour,
            TimePreset::OneDay,
        ];

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(
                // Toggle button
                div()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(if self.show_time_filters { theme.surface_raised } else { theme.surface })
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("时间")
                    )
            )
            .when(self.show_time_filters, |this| {
                this.children(
                    presets.iter().map(|preset| {
                        let is_selected = self.selected_time_preset == Some(*preset);
                        div()
                            .px(px(6.0))
                            .py(px(3.0))
                            .rounded(px(3.0))
                            .bg(if is_selected { theme.primary.opacity(0.2) } else { theme.surface })
                            .border(px(1.0))
                            .border_color(if is_selected { theme.primary } else { theme.border })
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(if is_selected { theme.primary } else { theme.text_muted })
                                    .text_xs()
                                    .child(preset.display())
                            )
                    })
                )
            })
    }

    /// Render progress indicator
    fn render_progress(&self) -> Div {
        let theme = &self.theme;
        let progress = &self.streaming_progress;
        let progress_pct = progress.percentage();

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child("接收中")
            )
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(progress.format_progress())
            )
            .child(
                // Progress bar
                div()
                    .flex_1()
                    .h(px(4.0))
                    .rounded(px(2.0))
                    .bg(theme.surface_raised)
                    .child(
                        div()
                            .w(px(progress_pct * 200.0))
                            .h(px(4.0))
                            .rounded(px(2.0))
                            .bg(theme.primary)
                    )
            )
    }

    /// Render query/stop button
    fn render_query_button(&self) -> Div {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .items_center()
            .justify_center()
            .px(px(16.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(if self.is_querying { theme.error } else { theme.primary })
            .cursor_pointer()
            .child(
                div()
                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                    .text_sm()
                    .child(if self.is_querying {
                        t.messages.stop.clone()
                    } else {
                        t.messages.query.clone()
                    })
            )
    }
}

impl IntoElement for MessageQueryTool {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .w_full()
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Main toolbar row
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .flex_wrap()
                    .child(self.render_partition_selector())
                    .child(self.render_mode_selector())
                    .child(self.render_max_messages())
                    .child(self.render_search_input())
                    .child(self.render_time_preset())
                    .child(self.render_query_button())
            )
            .when(self.is_querying, |this| {
                this.child(self.render_progress())
            })
    }
}

impl Clone for MessageQueryTool {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            selected_partition: self.selected_partition,
            partitions: self.partitions.clone(),
            query_mode: self.query_mode,
            max_messages: self.max_messages,
            search_keyword: self.search_keyword.clone(),
            show_time_filters: self.show_time_filters,
            selected_time_preset: self.selected_time_preset,
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            is_querying: self.is_querying,
            streaming_progress: self.streaming_progress.clone(),
            search_input: self.search_input.clone(),
        }
    }
}