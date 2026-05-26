# Kafka Manager GPUI Implementation

## Overview

This is a GPUI-based desktop application for managing Kafka clusters, built using the Zed editor's GPUI framework.

## Project Statistics

- **60 Rust source files**
- **~10,225 lines of code**
- **7 main views** + **15 UI components** + **5 context menus**

## Architecture

### Views (`src/ui/views/`)
| View | Description |
|------|-------------|
| ClustersView | Manage Kafka cluster connections |
| TopicsView | Browse and manage topics |
| MessagesView | Query messages with SSE streaming |
| ConsumerGroupsView | Monitor consumer group status |
| SchemaRegistryView | Manage Avro/Protobuf schemas |
| SettingsView | App preferences (theme/language) |
| FavoritesView | Saved favorite topics |

### Components (`src/ui/components/`)
| Component | Description |
|-----------|-------------|
| Modal | Base modal dialog container |
| Button | Styled button with hover states |
| Input | Text input field |
| Toast | Notification toast system |
| ClusterTreeNavigator | Tree view for clusters |
| ContextMenus | 5 types (cluster/topic/partition/topics-folder) |
| FavoriteButton | Toggle favorite status |
| TopicHistory | Recent topic query history |
| SentMessageHistory | Sent message records |
| CreateTopicDialog | Create new topic modal |
| DeleteTopicDialog | Delete topic confirmation |
| JsonEditor | JSON viewer with syntax highlighting |
| VirtualList | Efficient scrollable list |
| SendMessageModal | Send message to topic |
| MessageDetailPanel | Message details viewer |

### State Management (`src/state/`)
- **AppState**: Central data store
- **Router**: Navigation state
- **MessageBuffer**: Memory-efficient circular buffer
- **GlobalState**: Unified state entity
- **FavoritesState**: Favorite items management

### API Layer (`src/api/`)
- **Client**: HTTP client for backend
- **Types**: Request/response types
- **SSE**: Server-sent events streaming
- **SseStreamHandler**: Real-time message handler

## Key Features

### Memory Optimization
- Circular buffer with configurable limits
- Auto-evict when memory threshold reached
- Virtual scrolling for large lists

### SSE Streaming
- Real-time message query progress
- Batch event handling
- Error recovery

### i18n Support
- Chinese (zh) and English (en)
- Full translations for all views

### Keyboard Shortcuts
- Cmd+1-6: Navigate to views
- Cmd+Shift+B: Toggle sidebar
- Cmd+R: Refresh

## Build & Run

```bash
# Development
cargo build

# Release
cargo build --release

# Run
cargo run --release
```

## Dependencies

- **GPUI**: Zed editor's GPU-accelerated UI framework
- **Tokio**: Async runtime
- **Reqwest**: HTTP client
- **Serde**: JSON serialization
- **Chrono**: Time utilities

## Next Steps

1. Backend API integration
2. Real data fetching
3. Multi-platform packaging
4. Performance optimization
5. User testing

## License

MIT License