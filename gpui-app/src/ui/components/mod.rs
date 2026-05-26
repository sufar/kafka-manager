//! UI Components Module
//!
//! Contains reusable UI components.

pub mod modal;
pub mod button;
pub mod input;
pub mod toast;
pub mod cluster_tree_navigator;
pub mod confirm_dialog;
pub mod context_menus;
pub mod favorite_button;
pub mod topic_history;
pub mod sent_message_history;
pub mod create_topic_dialog;
pub mod delete_topic_dialog;
pub mod json_editor;
pub mod json_highlight_selector;
pub mod language_selector;
pub mod message_query_tool;
pub mod mobile_search_drawer;
pub mod mobile_card_view;
pub mod navigator_search;
pub mod navigator_status_bar;
pub mod topic_list_view;
pub mod topic_favorites;
pub mod topic_navigator;
pub mod virtual_list;
pub mod send_message_modal;
pub mod sse_stream_manager;
pub mod theme_selector;
pub mod message_detail_panel;

pub use json_highlight_selector::{JsonHighlightSelector, JsonHighlightTheme, ThemePreviewColors};
pub use language_selector::{LanguageSelector, LanguageOption};
pub use mobile_search_drawer::{MobileSearchDrawer, QuickAction, QuickActionIcon};
pub use mobile_card_view::{MobileMessageCardView, MOBILE_CARD_HEIGHT, render_mobile_message_card};
pub use message_query_tool::{MessageQueryTool, QueryMode, TimePreset, StreamingProgress, QueryParams};
pub use navigator_search::{NavigatorSearch, SearchResult, SearchItemType};
pub use navigator_status_bar::{NavigatorStatusBar, StatusItem, StatusItemType, ConnectionSummary, MessageStats};
pub use topic_list_view::{TopicListView, TopicTab, TopicConfigEntry, TopicStats};
pub use topic_navigator::{TopicNavigator, TopicNavItem, TopicSortMode};
pub use topic_favorites::{TopicFavorites, FavoriteTopicItem, FavoriteGroup};
pub use message_detail_panel::{MessageDetailPanel, MessageDetail};
pub use toast::{ToastManager, ToastManagerWithState, Toast, ToastPosition};
pub use crate::state::ToastType;
pub use favorite_button::FavoriteButton;
pub use cluster_tree_navigator::{ClusterTreeNavigator, ClusterTreeNavigatorLegacy};
pub use confirm_dialog::{ConfirmDialog, ConfirmVariant};
pub use json_editor::JsonEditor;
pub use theme_selector::{ThemeSelector, ThemeMode};
pub use sse_stream_manager::{SseStreamManager, StreamState};
pub use send_message_modal::{SendMessageModal, SendMessageForm};
pub use create_topic_dialog::CreateTopicDialog;
pub use delete_topic_dialog::DeleteTopicDialog;
pub use virtual_list::{VirtualList, VirtualListConfig, VirtualListState, VirtualListItem, SimpleItem, MessageItem, MessageColumnWidths, render_message_row};
pub use topic_history::{TopicHistory, TopicHistoryItem};
pub use sent_message_history::SentMessageHistory;
pub use context_menus::{
    ClusterContextMenu, ClusterContextMenuWithState, ClusterAction,
    TopicContextMenu, TopicContextMenuWithState, TopicAction,
    PartitionContextMenu, PartitionAction,
    TopicsFolderContextMenu, TopicsFolderAction,
};
pub use modal::{Modal, ModalContent, ModalHeader, ModalFooter};
pub use button::{Button, ButtonVariant};
pub use input::Input;