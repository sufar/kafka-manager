//! UI Views Module
//!
//! Contains all main view components for the application.

mod clusters_view;
mod topics_view;
mod messages_view;
mod consumer_groups_view;
mod settings_view;
mod schema_registry_view;
mod favorites_view;
mod topic_consumer_groups_view;

pub use clusters_view::ClustersView;
pub use topics_view::TopicsView;
pub use messages_view::MessagesView;
pub use consumer_groups_view::ConsumerGroupsView;
pub use settings_view::SettingsView;
pub use schema_registry_view::SchemaRegistryView;
pub use favorites_view::FavoritesView;
pub use topic_consumer_groups_view::TopicConsumerGroupsView;
