//! Theme Module
//!
//! Contains theme definitions for light/dark mode.

/// Theme colors and styles
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub is_dark: bool,
    pub background: gpui::Hsla,
    pub surface: gpui::Hsla,
    pub surface_raised: gpui::Hsla,
    pub text: gpui::Hsla,
    pub text_secondary: gpui::Hsla,
    pub text_muted: gpui::Hsla,
    pub primary: gpui::Hsla,
    pub primary_hover: gpui::Hsla,
    pub secondary: gpui::Hsla,
    pub accent: gpui::Hsla,
    pub success: gpui::Hsla,
    pub warning: gpui::Hsla,
    pub error: gpui::Hsla,
    pub border: gpui::Hsla,
    pub border_focused: gpui::Hsla,
}

impl Theme {
    /// Create dark theme
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            is_dark: true,
            background: gpui::rgb(0x1a1a2e).into(),      // Dark blue-purple background
            surface: gpui::rgb(0x16213e).into(),         // Surface color
            surface_raised: gpui::rgb(0x1f2937).into(), // Raised surface
            text: gpui::rgb(0xf8fafc).into(),           // White-ish text
            text_secondary: gpui::rgb(0xe2e8f0).into(), // Secondary text
            text_muted: gpui::rgb(0x94a3b8).into(),     // Muted text
            primary: gpui::rgb(0x8b5cf6).into(),        // Purple primary (matching Zed)
            primary_hover: gpui::rgb(0xa78bfa).into(),  // Lighter purple for hover
            secondary: gpui::rgb(0x6366f1).into(),      // Indigo secondary
            accent: gpui::rgb(0x06b6d4).into(),         // Cyan accent
            success: gpui::rgb(0x22c55e).into(),        // Green success
            warning: gpui::rgb(0xf59e0b).into(),        // Yellow warning
            error: gpui::rgb(0xef4444).into(),          // Red error
            border: gpui::rgb(0x374151).into(),         // Gray border
            border_focused: gpui::rgb(0x8b5cf6).into(), // Primary color border for focus
        }
    }

    /// Create light theme
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            is_dark: false,
            background: gpui::rgb(0xf8fafc).into(),     // Light gray background
            surface: gpui::rgb(0xffffff).into(),        // White surface
            surface_raised: gpui::rgb(0xf1f5f9).into(), // Lighter raised surface
            text: gpui::rgb(0x1e293b).into(),           // Dark text
            text_secondary: gpui::rgb(0x334155).into(), // Secondary text
            text_muted: gpui::rgb(0x64748b).into(),     // Muted text
            primary: gpui::rgb(0x8b5cf6).into(),        // Purple primary
            primary_hover: gpui::rgb(0x7c3aed).into(),  // Darker purple for hover
            secondary: gpui::rgb(0x6366f1).into(),      // Indigo secondary
            accent: gpui::rgb(0x0891b2).into(),         // Darker cyan accent
            success: gpui::rgb(0x16a34a).into(),        // Darker green
            warning: gpui::rgb(0xd97706).into(),        // Darker yellow
            error: gpui::rgb(0xdc2626).into(),          // Darker red
            border: gpui::rgb(0xe2e8f0).into(),         // Light border
            border_focused: gpui::rgb(0x8b5cf6).into(), // Primary color for focus
        }
    }
}