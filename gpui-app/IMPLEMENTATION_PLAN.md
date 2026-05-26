# GPUI 1:1 Implementation Plan

## Gap Analysis Summary

The Vue3 + Tauri2 version has:
- Dynamic cluster tree with real API data
- Expandable clusters with health indicators
- Virtual scrolling for topics/consumer groups
- Real navigation with query params
- Context menus integration
- Toast/Confirm dialogs
- Sidebar mode toggle (tree vs flat)
- Resizable sidebar
- Real API calls via Pinia stores

The GPUI version has:
- Static mock data
- No API integration
- Static navigation buttons
- No context menus integration
- No virtual scrolling data binding

## Implementation Priority

### P1 - Critical (Core Functionality)

1. **GlobalState as Entity with API**
   - Use `Entity<GlobalState>` pattern
   - Add `cx.spawn()` for async API calls
   - Load clusters from backend
   - Store cluster health status
   - Store topics/consumer groups per cluster

2. **LeftSidebar Integration**
   - Integrate ClusterTreeNavigator (tree mode)
   - Integrate TopicNavigator (flat mode)
   - Add sidebar mode toggle
   - Add resizable sidebar
   - Pass Entity<GlobalState> reference

3. **ClusterTreeNavigator Dynamic Data**
   - Remove mock data
   - Load clusters from GlobalState Entity
   - Health check indicators (green/red/yellow dots)
   - Expandable topics folder
   - Expandable consumer groups folder
   - Topic/Consumer group search
   - Click handlers for navigation

4. **Navigation with Query Params**
   - Router should support query params
   - Navigate to /messages?cluster=X&topic=Y
   - Navigate to /topics?cluster=X
   - State persistence

### P2 - High (User Experience)

5. **TopNavBar Actions**
   - Global search
   - Language toggle
   - Theme toggle
   - Share button
   - Tour start

6. **Context Menus**
   - Cluster context menu (view topics, refresh, test connection, disconnect, reconnect, delete)
   - Topic context menu (view messages, view details, send message, delete)
   - Partition context menu (view messages, send message)
   - Topics folder context menu (refresh, create topic, view all)

7. **Toast/Confirm Integration**
   - Global showToast method
   - Global showConfirm method
   - Success/error/warning/info types

### P3 - Medium (Polish)

8. **Virtual Scrolling**
   - Use VirtualList component
   - Bind to MessageBuffer
   - Bind to topic/consumer group lists

9. **Sidebar Resizing**
   - Add drag handle
   - Store width in settings

10. **Sidebar Mode Toggle**
    - Tree vs Flat toggle
    - Persist in settings

## Implementation Steps

### Step 1: GlobalState Entity with API

```rust
// In state/global_state.rs
pub struct GlobalState {
    pub api_client: ApiClient,
    pub clusters: Vec<Cluster>,
    pub cluster_health: HashMap<String, ClusterHealth>,
    pub topics: HashMap<String, Vec<Topic>>,
    pub consumer_groups: HashMap<String, Vec<ConsumerGroup>>,
    // ... existing fields
}

impl GlobalState {
    pub fn load_clusters(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            let clusters = this.read(&cx).api_client.cluster_api.list().await;
            this.update(&mut cx, |state, cx| {
                state.clusters = clusters.unwrap_or_default();
                cx.notify();
            })
        }).detach();
    }
}
```

### Step 2: ClusterTreeNavigator Integration

```rust
// In components/cluster_tree_navigator.rs
pub struct ClusterTreeNavigator {
    state: Entity<GlobalState>,
    expanded_clusters: HashSet<String>,
    selected_cluster: Option<String>,
    selected_topic: Option<(String, String)>,
}

impl Render for ClusterTreeNavigator {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        // Render from state.clusters, not mock data
    }
}
```

### Step 3: Navigation Actions

```rust
// Define navigation actions with parameters
actions!(navigation, [
    NavigateToMessages(NavigateToMessagesAction),
    NavigateToTopics(NavigateToTopicsAction),
]);

pub struct NavigateToMessagesAction {
    pub cluster: String,
    pub topic: String,
}
```

### Step 4: LeftSidebar Integration

```rust
// In layout/left_sidebar.rs
pub struct LeftSidebar {
    state: Entity<GlobalState>,
    sidebar_mode: SidebarMode, // Tree or Flat
}

impl LeftSidebar {
    pub fn new(state: Entity<GlobalState>, ...) -> Self {
        Self { state, ... }
    }
}
```

## Files to Modify

1. `src/state/global_state.rs` - Add Entity pattern with API calls
2. `src/app.rs` - Pass Entity to all components
3. `src/ui/layout/left_sidebar.rs` - Integrate ClusterTreeNavigator
4. `src/ui/components/cluster_tree_navigator.rs` - Dynamic data from Entity
5. `src/router/` - Add query param support
6. `src/ui/layout/top_nav_bar.rs` - Add search, toggles, actions
7. `src/ui/components/context_menus/*.rs` - Integrate with real data

## Testing

1. Start backend server: `./start-server.sh`
2. Run GPUI app: `cargo run`
3. Verify cluster data loads
4. Verify navigation works
5. Verify context menus appear
6. Verify toast notifications

---
*Plan created: 2026-05-25*