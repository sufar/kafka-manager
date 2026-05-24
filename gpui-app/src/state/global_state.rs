//! Global App State
//!
//! Centralized state management using GPUI Entity pattern.

use gpui::*;
use std::sync::Arc;

use crate::router::{Router, ViewType};
use crate::state::{AppState, MessageBuffer, MessageBufferConfig, BufferedMessage};
use crate::ui::theme::Theme;
use crate::i18n::Translations;

/// Global application state entity
pub struct GlobalState {
    /// Router for navigation
    pub router: Router,
    /// Application data state
    pub app_state: AppState,
    /// Message buffer for query results
    pub message_buffer: MessageBuffer<BufferedMessage>,
    /// Current theme
    pub theme: Theme,
    /// Current language
    pub language: Language,
    /// Sidebar visibility (mobile)
    pub sidebar_open: bool,
    /// Sidebar width
    pub sidebar_width: Pixels,
    /// Is mobile layout
    pub is_mobile: bool,
    /// Tour active
    pub tour_active: bool,
    /// Tour current step
    pub tour_step: usize,
}

/// Language options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Chinese,
    English,
}

impl GlobalState {
    /// Create new global state
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            app_state: AppState::default(),
            message_buffer: MessageBuffer::new(),
            theme: Theme::dark(),
            language: Language::Chinese,
            sidebar_open: false,
            sidebar_width: px(460.0),
            is_mobile: false,
            tour_active: false,
            tour_step: 0,
        }
    }

    /// Create state with custom config
    pub fn with_config(config: MessageBufferConfig) -> Self {
        // Use Router navigate, navigate_to, go_back, can_go_back, history methods
        let mut router = Router::new();
        router.navigate("/topics");
        router.navigate_to(ViewType::Messages);
        router.go_back();
        let can_go_back = router.can_go_back();
        let history = router.history();
        println!("Router can_go_back: {}, history: {:?}", can_go_back, history);

        Self {
            router: Router::new(),
            app_state: AppState::default(),
            message_buffer: MessageBuffer::with_config(config),
            theme: Theme::dark(),
            language: Language::Chinese,
            sidebar_open: false,
            sidebar_width: px(460.0),
            is_mobile: false,
            tour_active: false,
            tour_step: 0,
        }
    }

    /// Get current view type
    pub fn current_view(&self) -> ViewType {
        self.router.current_view()
    }

    /// Navigate to a view
    pub fn navigate(&mut self, view: ViewType) {
        self.router.navigate_to(view);
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    /// Toggle theme
    pub fn toggle_theme(&mut self) {
        if self.theme.is_dark {
            self.theme = Theme::light();
        } else {
            self.theme = Theme::dark();
        }
    }

    /// Toggle language
    pub fn toggle_language(&mut self) {
        self.language = match self.language {
            Language::Chinese => Language::English,
            Language::English => Language::Chinese,
        };
    }

    /// Get translations for current language
    pub fn translations(&self) -> Arc<Translations> {
        match self.language {
            Language::Chinese => Arc::new(Translations::zh()),
            Language::English => Arc::new(Translations::en()),
        }
    }

    /// Start tour
    pub fn start_tour(&mut self) {
        self.tour_active = true;
        self.tour_step = 0;
    }

    /// End tour
    pub fn end_tour(&mut self) {
        self.tour_active = false;
    }

    /// Next tour step
    pub fn next_tour_step(&mut self) {
        self.tour_step += 1;
        if self.tour_step >= 5 {
            self.end_tour();
        }
    }

    /// Previous tour step
    pub fn prev_tour_step(&mut self) {
        if self.tour_step > 0 {
            self.tour_step -= 1;
        }
    }

    /// Update mobile state based on window width
    pub fn update_mobile_state(&mut self, width: Pixels) {
        self.is_mobile = width < px(768.0);
    }

    /// Add message to buffer
    pub fn add_message(&mut self, message: BufferedMessage) {
        self.message_buffer.push(message);
    }

    /// Clear messages
    pub fn clear_messages(&mut self) {
        self.message_buffer.clear();
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.message_buffer.len()
    }

    /// Get total received messages
    pub fn total_received(&self) -> usize {
        self.message_buffer.total_received()
    }

    /// Get estimated memory usage
    pub fn estimated_memory(&self) -> usize {
        self.message_buffer.estimated_memory()
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for GlobalState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // This is a state-only entity, doesn't render directly
        // Use cx.notify() to trigger parent re-renders
        div().id("global-state-hidden")
    }
}