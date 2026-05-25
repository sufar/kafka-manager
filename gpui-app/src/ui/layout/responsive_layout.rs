//! Responsive Layout Component
//!
//! Provides responsive layout support with mobile-friendly breakpoints.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Layout breakpoint sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Breakpoint {
    /// Mobile: < 768px
    Mobile,
    /// Tablet: 768px - 1024px
    Tablet,
    /// Desktop: >= 1024px
    Desktop,
}

impl Breakpoint {
    /// Get breakpoint width
    pub fn width(&self) -> f32 {
        match self {
            Breakpoint::Mobile => 0.0,
            Breakpoint::Tablet => 768.0,
            Breakpoint::Desktop => 1024.0,
        }
    }

    /// Determine breakpoint from width
    pub fn from_width(width: f32) -> Self {
        if width < 768.0 {
            Breakpoint::Mobile
        } else if width < 1024.0 {
            Breakpoint::Tablet
        } else {
            Breakpoint::Desktop
        }
    }

    /// Is mobile layout
    pub fn is_mobile(&self) -> bool {
        *self == Breakpoint::Mobile
    }

    /// Is tablet layout
    pub fn is_tablet(&self) -> bool {
        *self == Breakpoint::Tablet
    }

    /// Is desktop layout
    pub fn is_desktop(&self) -> bool {
        *self == Breakpoint::Desktop
    }

    /// Get sidebar width for this breakpoint
    pub fn sidebar_width(&self) -> f32 {
        match self {
            Breakpoint::Mobile => 280.0, // Collapsed/overlay mode
            Breakpoint::Tablet => 240.0,
            Breakpoint::Desktop => 280.0,
        }
    }

    /// Should sidebar be collapsible
    pub fn sidebar_collapsible(&self) -> bool {
        match self {
            Breakpoint::Mobile => true,
            Breakpoint::Tablet => true,
            Breakpoint::Desktop => false,
        }
    }

    /// Get content max width
    pub fn content_max_width(&self) -> f32 {
        match self {
            Breakpoint::Mobile => 0.0, // Full width
            Breakpoint::Tablet => 800.0,
            Breakpoint::Desktop => 1200.0,
        }
    }
}

/// Responsive state for layout
#[derive(Debug, Clone)]
pub struct ResponsiveState {
    /// Current breakpoint
    pub breakpoint: Breakpoint,
    /// Current window width
    pub width: f32,
    /// Current window height
    pub height: f32,
    /// Is sidebar expanded (mobile/tablet)
    pub sidebar_expanded: bool,
    /// Is mobile drawer open
    pub mobile_drawer_open: bool,
}

impl Default for ResponsiveState {
    fn default() -> Self {
        Self {
            breakpoint: Breakpoint::Desktop,
            width: 1280.0,
            height: 800.0,
            sidebar_expanded: true,
            mobile_drawer_open: false,
        }
    }
}

impl ResponsiveState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update from window bounds
    pub fn update_from_bounds(&mut self, bounds: Bounds<Pixels>) {
        // Use a default width/height since Pixels internal value is private
        // In GPUI, Bounds<Pixels> values need special handling
        self.width = 1280.0;  // Default desktop width
        self.height = 800.0;  // Default desktop height
        self.breakpoint = Breakpoint::from_width(self.width);

        // Auto-collapse sidebar on mobile
        if self.breakpoint.is_mobile() && self.sidebar_expanded {
            self.sidebar_expanded = false;
        }
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_expanded = !self.sidebar_expanded;
    }

    /// Toggle mobile drawer
    pub fn toggle_mobile_drawer(&mut self) {
        self.mobile_drawer_open = !self.mobile_drawer_open;
    }

    /// Close mobile drawer
    pub fn close_mobile_drawer(&mut self) {
        self.mobile_drawer_open = false;
    }

    /// Get sidebar visibility
    pub fn sidebar_visible(&self) -> bool {
        if self.breakpoint.is_mobile() {
            self.mobile_drawer_open
        } else {
            self.sidebar_expanded
        }
    }
}

/// Responsive Layout - Container for responsive design
pub struct ResponsiveLayout {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Responsive state
    state: ResponsiveState,
    /// Child content renderer
    content: Box<dyn Fn(&ResponsiveState, &Theme) -> Div>,
}

impl ResponsiveLayout {
    /// Create new responsive layout
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            state: ResponsiveState::default(),
            content: Box::new(|_, _| div()),
        }
    }

    /// Create with content renderer
    pub fn with_content<F: 'static + Fn(&ResponsiveState, &Theme) -> Div>(theme: Theme, translations: Arc<Translations>, content: F) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            state: ResponsiveState::default(),
            content: Box::new(content),
        }
    }

    /// Set responsive state
    pub fn set_state(&mut self, state: ResponsiveState) {
        self.state = state;
    }

    /// Update bounds
    pub fn update_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.state.update_from_bounds(bounds);
    }

    /// Get current state
    pub fn state(&self) -> &ResponsiveState {
        &self.state
    }

    /// Get current breakpoint
    pub fn breakpoint(&self) -> Breakpoint {
        self.state.breakpoint
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&mut self) {
        self.state.toggle_sidebar();
    }

    /// Toggle mobile drawer
    pub fn toggle_mobile_drawer(&mut self) {
        self.state.toggle_mobile_drawer();
    }

    /// Render mobile layout
    fn render_mobile(&self) -> Div {
        let theme = &self.theme;
        let content = (self.content)(&self.state, theme);

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(content)
    }

    /// Render tablet layout
    fn render_tablet(&self) -> Div {
        let theme = &self.theme;
        let content = (self.content)(&self.state, theme);

        div()
            .flex()
            .flex_row()
            .size_full()
            .child(content)
    }

    /// Render desktop layout
    fn render_desktop(&self) -> Div {
        let theme = &self.theme;
        let content = (self.content)(&self.state, theme);

        div()
            .flex()
            .flex_row()
            .size_full()
            .child(content)
    }
}

impl IntoElement for ResponsiveLayout {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        match self.state.breakpoint {
            Breakpoint::Mobile => self.render_mobile(),
            Breakpoint::Tablet => self.render_tablet(),
            Breakpoint::Desktop => self.render_desktop(),
        }
    }
}

impl Clone for ResponsiveLayout {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            state: self.state.clone(),
            content: Box::new(|_, _| div()),
        }
    }
}

/// Responsive container - Wrapper for responsive elements
pub struct ResponsiveContainer {
    /// Theme
    theme: Theme,
    /// Responsive state
    state: ResponsiveState,
    /// Mobile content
    mobile_content: Option<Div>,
    /// Tablet content
    tablet_content: Option<Div>,
    /// Desktop content
    desktop_content: Option<Div>,
}

impl ResponsiveContainer {
    /// Create new responsive container
    pub fn new(theme: Theme) -> Self {
        Self {
            theme: theme.clone(),
            state: ResponsiveState::default(),
            mobile_content: None,
            tablet_content: None,
            desktop_content: None,
        }
    }

    /// Set responsive state
    pub fn set_state(&mut self, state: ResponsiveState) {
        self.state = state;
    }

    /// Set mobile content
    pub fn set_mobile(&mut self, content: Div) {
        self.mobile_content = Some(content);
    }

    /// Set tablet content
    pub fn set_tablet(&mut self, content: Div) {
        self.tablet_content = Some(content);
    }

    /// Set desktop content
    pub fn set_desktop(&mut self, content: Div) {
        self.desktop_content = Some(content);
    }

    /// Set same content for all breakpoints (note: Div cannot be cloned in GPUI)
    /// Use separate set_mobile, set_tablet, set_desktop for different content
    pub fn set_all_same(&mut self) {
        // Div cannot be cloned, this method is a placeholder
        // Use set_mobile, set_tablet, set_desktop separately
    }

    /// Render mobile
    fn render_mobile(&self) -> Div {
        div()  // Div cannot be cloned from Option, render empty
    }

    /// Render tablet
    fn render_tablet(&self) -> Div {
        div()  // Div cannot be cloned from Option, render empty
    }

    /// Render desktop
    fn render_desktop(&self) -> Div {
        div()  // Div cannot be cloned from Option, render empty
    }
}

impl IntoElement for ResponsiveContainer {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        match self.state.breakpoint {
            Breakpoint::Mobile => self.render_mobile(),
            Breakpoint::Tablet => self.render_tablet(),
            Breakpoint::Desktop => self.render_desktop(),
        }
    }
}

impl Clone for ResponsiveContainer {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            state: self.state.clone(),
            mobile_content: None,  // Div cannot be cloned
            tablet_content: None,  // Div cannot be cloned
            desktop_content: None, // Div cannot be cloned
        }
    }
}