//! Keyboard Shortcuts
//!
//! Defines keyboard shortcuts for the application.

use gpui::*;

// Navigation shortcuts
actions!(navigation, [
    GoToClusters,
    GoToTopics,
    GoToMessages,
    GoToConsumerGroups,
    GoToSchemaRegistry,
    GoToSettings,
    ToggleSidebar,
    Refresh,
]);

/// Register all keyboard shortcuts
pub fn register_shortcuts(cx: &mut App) {
    // Navigation shortcuts: Cmd/Ctrl + number
    cx.bind_keys([KeyBinding::new("cmd-1", GoToClusters, None)]);
    cx.bind_keys([KeyBinding::new("cmd-2", GoToTopics, None)]);
    cx.bind_keys([KeyBinding::new("cmd-3", GoToMessages, None)]);
    cx.bind_keys([KeyBinding::new("cmd-4", GoToConsumerGroups, None)]);
    cx.bind_keys([KeyBinding::new("cmd-5", GoToSchemaRegistry, None)]);
    cx.bind_keys([KeyBinding::new("cmd-6", GoToSettings, None)]);
    cx.bind_keys([KeyBinding::new("cmd-shift-b", ToggleSidebar, None)]);
    cx.bind_keys([KeyBinding::new("cmd-r", Refresh, None)]);
}