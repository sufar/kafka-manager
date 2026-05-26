//! Favorites State
//!
//! State management for favorites functionality.

/// Favorite group
#[derive(Debug, Clone)]
pub struct FavoriteGroup {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub item_count: i32,
}

/// Favorite item
#[derive(Debug, Clone)]
pub struct FavoriteItem {
    pub id: i32,
    pub group_id: i32,
    pub cluster_id: String,
    pub topic_name: String,
    pub description: Option<String>,
    pub created_at: i64,
}

/// Favorites state
#[derive(Debug, Clone, Default)]
pub struct FavoritesState {
    pub groups: Vec<FavoriteGroup>,
    pub items: Vec<FavoriteItem>,
    pub selected_group_id: Option<i32>,
}

impl FavoritesState {
    /// Create new favorites state
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            items: Vec::new(),
            selected_group_id: None,
        }
    }

    /// Add a group
    pub fn add_group(&mut self, group: FavoriteGroup) {
        self.groups.push(group);
    }

    /// Remove a group
    pub fn remove_group(&mut self, id: i32) {
        self.groups.retain(|g| g.id != id);
        self.items.retain(|i| i.group_id != id);
    }

    /// Add a favorite item
    pub fn add_item(&mut self, item: FavoriteItem) {
        // Check if already exists
        if !self.items.iter().any(|i| i.cluster_id == item.cluster_id && i.topic_name == item.topic_name) {
            let group_id = item.group_id;
            self.items.push(item);
            // Update item count for group
            if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) {
                group.item_count += 1;
            }
        }
    }

    /// Remove a favorite item
    pub fn remove_item(&mut self, cluster_id: &str, topic_name: &str) {
        if let Some(item) = self.items.iter().find(|i| i.cluster_id == cluster_id && i.topic_name == topic_name) {
            let group_id = item.group_id;
            self.items.retain(|i| i.cluster_id != cluster_id || i.topic_name != topic_name);
            // Update item count for group
            if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) {
                group.item_count -= 1;
            }
        }
    }

    /// Check if topic is favorite
    pub fn is_favorite(&self, cluster_id: &str, topic_name: &str) -> bool {
        self.items.iter().any(|i| i.cluster_id == cluster_id && i.topic_name == topic_name)
    }

    /// Get items in a group
    pub fn get_items_in_group(&self, group_id: i32) -> Vec<&FavoriteItem> {
        self.items.iter().filter(|i| i.group_id == group_id).collect()
    }

    /// Get all groups
    pub fn get_groups(&self) -> &[FavoriteGroup] {
        &self.groups
    }
}