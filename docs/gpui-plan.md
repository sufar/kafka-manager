# Kafka Manager GPUI Migration - 迁移计划文档

> 本文档规划 Tauri2 + Vue3 前端迁移到 GPUI 的详细实施步骤。
> 
> 生成日期: 2026-05-24
> 项目版本: 1.1.47

---

## 概述

### 目标
将 Kafka Manager 的前端从 Tauri2 + Vue3 + DaisyUI 迁移到纯 Rust GPUI 框架。

### 当前架构
```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri 2 Desktop App                      │
├─────────────────────────────────────────────────────────────┤
│  Vue3 + TypeScript + Vite + Tailwind + DaisyUI             │
│  ├── ui/src/                                                │
│  │   ├── views/ (8 个页面组件)                               │
│  │   ├── components/ (30+ 个组件)                            │
│  │   ├── stores/ (Pinia 状态管理)                            │
│  │   ├── api/ (HTTP + SSE 客户端)                            │
│  │   ├── router/ (Vue Router)                               │
│  │   ├── i18n/ (国际化)                                      │
│  │   └── layouts/ (布局组件)                                 │
├─────────────────────────────────────────────────────────────┤
│  src-tauri/ (Tauri 配置 + 调用后端)                           │
├─────────────────────────────────────────────────────────────┤
│  src/ (Rust 后端: axum + rdkafka + sqlx)                     │
└─────────────────────────────────────────────────────────────┘
```

### 目标架构
```
┌─────────────────────────────────────────────────────────────┐
│                    GPUI Desktop App                          │
├─────────────────────────────────────────────────────────────┤
│  gpui-app/                                                   │
│  ├── src/                                                    │
│  │   ├── app.rs (主应用入口)                                  │
│  │   ├── ui/ (所有 UI 组件)                                   │
│  │   │   ├── views/                                          │
│  │   │   ├── components/                                     │
│  │   │   ├── layouts/                                        │
│  │   ├── state/ (应用状态)                                    │
│  │   ├── api/ (HTTP + SSE 客户端)                            │
│  │   ├── theme/ (主题系统)                                    │
│  │   └── i18n/ (国际化)                                       │
├─────────────────────────────────────────────────────────────┤
│  src/ (Rust 后端: axum + rdkafka + sqlx) ← 保留不变          │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: 基础框架搭建 (预估 2-3 周)

### 1.1 GPUI 项目初始化

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 1.1.1 | 创建 GPUI 项目结构 | 新建 `gpui-app/` 目录，Cargo.toml 配置 | 1 天 |
| 1.1.2 | 配置 GPUI 依赖 | 添加 gpui crate (需从 Zed 源码或 crates.io) | 1 天 |
| 1.1.3 | 实现主窗口框架 | Window + App 基础结构 | 2 天 |
| 1.1.4 | 实现主题系统 | Light/Dark 模式切换 | 2 天 |
| 1.1.5 | 实现基础布局组件 | Header + Sidebar + MainContent | 3 天 |

**关键决策:**

- GPUI 版本选择: 
  - 选项 A: 直接使用 Zed 源码中的 gpui crate
  - 选项 B: 等 gpui 发布到 crates.io (目前尚未发布)
  - **推荐**: 先从 Zed 源码提取 gpui，作为本地依赖

- 布局模式:
  - GPUI 使用即时模式 GUI，需要重新设计组件架构
  - 建议: 借鉴 Zed 的 panel system 设计

### 1.2 API 客户端移植

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 1.2.1 | HTTP 客户端 | 使用 `reqwest` 或 `ureq` 实现 POST `/api` | 2 天 |
| 1.2.2 | SSE 客户端 | 实现流式消息接收 | 3 天 |
| 1.2.3 | 错误处理 | 统一错误类型 | 1 天 |
| 1.2.4 | 请求取消 | 实现 AbortController 类似机制 | 1 天 |
| 1.2.5 | 后端就绪检查 | 启动时等待后端就绪 | 1 天 |

**API 客户端设计:**

```rust
// gpui-app/src/api/client.rs
pub struct ApiClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl ApiClient {
    pub async fn request<T: DeserializeOwned>(&self, method: &str, body: Value) -> Result<T>;
    pub async fn get_messages_stream(&self, ...) -> impl Stream<Item = Message>;
    // ... 所有 API 方法
}
```

### 1.3 状态管理设计

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 1.3.1 | 应用状态结构 | 设计 `AppState` struct | 1 天 |
| 1.3.2 | 集群状态 | ClusterStore 对应 | 1 天 |
| 1.3.3 | 主题状态 | ThemeStore 对应 | 1 天 |
| 1.3.4 | 语言状态 | LanguageStore 对应 | 1 天 |

**状态管理设计:**

GPUI 使用 Model/View 模式，状态通过 `Model<T>` 管理：

```rust
// gpui-app/src/state/app_state.rs
pub struct AppState {
    pub clusters: Vec<Cluster>,
    pub cluster_groups: Vec<ClusterGroup>,
    pub connections: HashMap<String, ConnectionStatus>,
    pub selected_cluster: Option<String>,
    pub theme: Theme,
    pub language: Language,
}

// 在 GPUI 中使用 Model<AppState>
pub fn build_app(cx: &mut AppContext) {
    let state = cx.new_model(|cx| AppState::default());
    // ...
}
```

---

## Phase 2: 核心功能实现 (预估 4-5 周)

### 2.1 集群管理模块

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 2.1.1 | 集群列表视图 | Cards Grid 展示 | 3 天 |
| 2.1.2 | 创建/编辑集群 Modal | 表单 + 验证 | 3 天 |
| 2.1.3 | 删除集群确认 | Confirm Dialog | 1 天 |
| 2.1.4 | 测试连接按钮 | 测试 + 状态显示 | 2 天 |
| 2.1.5 | 集群分组管理 | Modal + CRUD | 2 天 |
| 2.1.6 | 连接状态徽章 | connected/disconnected/error | 1 天 |
| 2.1.7 | 分组筛选标签 | 横向滚动 | 1 天 |

**组件设计:**

```rust
// gpui-app/src/ui/views/clusters_view.rs
pub struct ClustersView {
    clusters: Model<Vec<Cluster>>,
    groups: Model<Vec<ClusterGroup>>,
    selected_group: Option<i32>,
    // Modal 状态
    create_modal_open: bool,
    editing_cluster: Option<Cluster>,
    // ...
}

impl Render for ClustersView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(self.render_group_selector(cx))
            .child(self.render_clusters_grid(cx))
            // Modal 在需要时渲染
    }
}
```

### 2.2 Topic 管理模块

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 2.2.1 | Topic 列表视图 | Table + 虚拟滚动 | 4 天 |
| 2.2.2 | Topic 搜索 | 本地筛选 + 防抖 | 2 天 |
| 2.2.3 | 创建 Topic Modal | 名称/分区/副本/配置 | 3 天 |
| 2.2.4 | 删除 Topic Dialog | 确认弹窗 | 1 天 |
| 2.2.5 | 刷新 Topic | 按钮触发 | 1 天 |
| 2.2.6 | 收藏按钮 | FavoriteButton | 1 天 |

**虚拟滚动挑战:**

GPUI 没有内置虚拟滚动，需要自己实现或使用 List 组件的懒加载特性：

```rust
// 参考 Zed 的项目搜索结果列表
pub struct TopicList {
    topics: Vec<Topic>,
    visible_range: Range<usize>,
    scroll_offset: f32,
}

impl Render for TopicList {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .overflow_y_scroll()
            .on_scroll(move |event, cx| {
                // 更新 visible_range
            })
            .children(self.visible_topics().map(|topic| self.render_row(topic)))
    }
}
```

### 2.3 消息查询模块 (最复杂)

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 2.3.1 | 消息工具栏 | 分区/模式/数量/搜索/时间筛选 | 3 天 |
| 2.3.2 | SSE 流式接收 | 流式消息处理 | 4 天 |
| 2.3.3 | 消息列表 | 虚拟滚动表格 | 5 天 |
| 2.3.4 | 列宽调整 | 拖动调整列宽 | 2 天 |
| 2.3.5 | 消息详情面板 | 底部可调整面板 | 3 天 |
| 2.3.6 | JSON 高亮 | 模板化语法高亮 | 3 天 |
| 2.3.7 | Value 格式切换 | json/raw/hex | 1 天 |
| 2.3.8 | 发送消息 Modal | Partition/Key/Value/Headers | 3 天 |
| 2.3.9 | 导出消息 | 文件保存 | 2 天 |
| 2.3.10 | 时间戳排序 | asc/desc/null | 1 天 |
| 2.3.11 | 流式进度显示 | 进度条 + 数量 | 1 天 |
| 2.3.12 | 键盘导航 | 方向键选择消息 | 1 天 |
| 2.3.13 | 详情搜索 | Ctrl+F 高亮 | 2 天 |

**SSE 流式设计:**

```rust
// gpui-app/src/api/sse.rs
pub struct MessageStream {
    abort_handle: AbortHandle,
}

impl MessageStream {
    pub fn start(&self, params: QueryParams) -> impl Stream<Item = StreamEvent> {
        // reqwest + SSE 解析
    }
    
    pub fn stop(&self) {
        self.abort_handle.abort();
    }
}

// StreamEvent types
pub enum StreamEvent {
    Start { partitions: usize, total_target: usize },
    Batch { messages: Vec<Message>, progress: usize, total: usize },
    Order { sort: String },
    Complete { actual_total: usize },
    Error { message: String },
}
```

### 2.4 Consumer Group 模块

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 2.4.1 | Consumer Group 详情视图 | Group 状态 + Topics | 2 天 |
| 2.4.2 | Offset 表格 | 分区/偏移量/Lag | 3 天 |
| 2.4.3 | 列宽调整 | 拖动调整 | 1 天 |
| 2.4.4 | 重置 Offset Modal | Topic/Partition/ResetTo 选择 | 3 天 |
| 2.4.5 | 删除 Consumer Group | 确认弹窗 | 1 天 |
| 2.4.6 | 状态徽章 | 状态颜色编码 | 1 天 |
| 2.4.7 | Lag 颜色编码 | 0/<1000/>1000 | 1 天 |

---

## Phase 3: 高级功能实现 (预估 3-4 周)

### 3.1 Schema Registry 模块

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 3.1.1 | Schema Registry 配置 | URL + 认证 Modal | 2 天 |
| 3.1.2 | Subjects 列表 | 表格展示 | 2 天 |
| 3.1.3 | Schema 查看 Modal | JSON 内容展示 | 2 天 |
| 3.1.4 | 注册 Schema Modal | Subject + Type + JSON | 3 天 |
| 3.1.5 | 兼容性测试 | 按钮触发 | 1 天 |
| 3.1.6 | 删除 Schema | 确认弹窗 | 1 天 |

### 3.2 收藏和历史模块

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 3.2.1 | 收藏分组列表 | 可折叠列表 | 2 天 |
| 3.2.2 | 分组 CRUD Modal | 创建/编辑/删除 | 2 天 |
| 3.2.3 | 收藏项 CRUD | 添加/删除 | 2 天 |
| 3.2.4 | 导航跳转 | 点击跳转消息页面 | 1 天 |
| 3.2.5 | Topic 历史 | 浏览历史列表 | 1 天 |
| 3.2.6 | 发送历史 | SentMessageHistory 面板 | 2 天 |

### 3.3 设置页面

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 3.3.1 | 语言切换 | 中文/英文 | 1 天 |
| 3.3.2 | 主题切换 | Dark/Light | 1 天 |
| 3.3.3 | 侧边栏模式 | tree/flat toggle | 1 天 |
| 3.3.4 | 系统托盘设置 | 关闭按钮行为 | 2 天 |
| 3.3.5 | 开机自启动 | Windows 注册表 | 2 天 |
| 3.3.6 | 版本显示 | Cargo.toml 版本 | 1 天 |
| 3.3.7 | 检查更新 | GitHub Release API | 2 天 |
| 3.3.8 | 下载更新 | 后台下载 + 进度 | 3 天 |
| 3.3.9 | 导入导出 | JSON 文件 | 2 天 |
| 3.3.10 | JSON 高亮模板 | 模板选择器 | 2 天 |
| 3.3.11 | 日志查看 | 隐藏功能 + Modal | 1 天 |

### 3.4 树形导航

**任务列表:**

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 3.4.1 | 树形结构组件 | 可折叠树 | 4 天 |
| 3.4.2 | 集群节点 | Cluster + 健康状态 | 2 天 |
| 3.4.3 | Topics 文件夹 | 可折叠文件夹 | 2 天 |
| 3.4.4 | Topic 节点 | Topic + 收藏图标 | 2 天 |
| 3.4.5 | Partition 节点 | 分区节点 | 1 天 |
| 3.4.6 | 搜索框 | Topic 搜索 | 2 天 |
| 3.4.7 | 右键菜单 | 各节点上下文菜单 | 3 天 |
| 3.4.8 | 展开/折叠控制 | 单独/批量 | 1 天 |
| 3.4.9 | 高亮选中 | 当前选中 Topic 高亮 | 1 天 |

---

## Phase 4: 优化和测试 (预估 2 周)

### 4.1 性能优化

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 4.1.1 | 虚拟滚动优化 | 大列表性能 | 2 天 |
| 4.1.2 | 内存优化 | 消息列表内存管理 | 2 天 |
| 4.1.3 | 渲染优化 | 减少不必要的重渲染 | 2 天 |
| 4.1.4 | SSE 流优化 | 批量更新频率调整 | 1 天 |

### 4.2 国际化完善

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 4.2.1 | 翻译迁移 | Vue translations.ts → Rust | 2 天 |
| 4.2.2 | 语言切换 UI | 所有文本动态切换 | 2 天 |
| 4.2.3 | 语言持久化 | localStorage 保存 | 1 天 |

### 4.3 测试

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 4.3.1 | 功能测试 | 所有功能点测试 | 3 天 |
| 4.3.2 | 边界测试 | 错误处理测试 | 2 天 |
| 4.3.3 | 性能测试 | 大数据量测试 | 2 天 |
| 4.3.4 | 回归测试 | 与 Vue 版本对比 | 2 天 |

---

## Phase 5: 发布准备 (预估 1 周)

| ID | 任务 | 描述 | 预估时间 |
|----|------|------|----------|
| 5.1 | 构建配置 | Release 构建优化 | 2 天 |
| 5.2 | 打包发布 | Windows/macOS/Linux | 2 天 |
| 5.3 | 文档更新 | README 更新 | 1 天 |
| 5.4 | 版本发布 | GitHub Release | 1 天 |

---

## 总时间预估

| Phase | 预估时间 |
|-------|----------|
| Phase 1: 基础框架 | 2-3 周 |
| Phase 2: 核心功能 | 4-5 周 |
| Phase 3: 高级功能 | 3-4 周 |
| Phase 4: 优化测试 | 2 周 |
| Phase 5: 发布准备 | 1 周 |
| **总计** | **12-15 周 (约 3-4 个月)** |

---

## 风险和挑战

### 高风险项

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| GPUI 未正式发布 | API 不稳定、文档缺失 | 从 Zed 源码学习，参考 Zed 组件实现 |
| 虚拟滚动无内置支持 | 大数据量性能问题 | 自行实现或借鉴 Zed list 组件 |
| 复杂表格组件 | 列宽调整、排序、筛选 | 参考 Zed 的 project search panel |
| Modal 系统复杂 | 多层嵌套 Modal | 设计统一 Modal 框架 |
| SSE 流式处理 | GPUI 异步处理模式 | 使用 async channels + Model updates |

### 技术债务风险

| 项目 | 当前 Vue 实现 | GPUI 挑战 |
|------|--------------|----------|
| DaisyUI 组件库 | 丰富的预设样式 | 需要从零构建所有样式 |
| Tailwind CSS | 快速样式开发 | GPUI 有自己的样式系统 |
| Vue Router | 声明式路由 | GPUI 需要手动管理视图切换 |
| Pinia Store | 响应式状态管理 | GPUI Model + View 模式 |

---

## 里程碑计划

| Milestone | 目标 | 完成标准 |
|-----------|------|----------|
| M1 | 基础框架 | 空窗口 + 主题切换 + 基础布局 |
| M2 | 集群管理 | 集群 CRUD + 连接测试 + 分组管理 |
| M3 | Topic + 消息查询 | Topic CRUD + SSE 消息查询 + 发送 |
| M4 | Consumer Group | Offset 查看 + 重置 + 删除 |
| M5 | 高级功能 | Schema Registry + 收藏 + 设置 |
| M6 | 完整功能 | 所有功能迁移完成 + 测试通过 |
| M7 | 发布 | 打包 + 发布 |

---

## 开发建议

### 开发顺序优先级

1. **先搭建骨架**: 主窗口 + 布局 + 主题
2. **先做简单功能**: 集群管理（表单为主）
3. **逐步增加复杂度**: Topic → 消息 → Consumer Group
4. **最后做高级功能**: Schema Registry + 设置

### 学习资源

- Zed 源码: https://github.com/zed-industries/zed
- GPUI 文档: 在 Zed 源码 `crates/gpui/` 目录
- Zed 组件参考: `crates/ui/` 目录中的组件实现

### 推荐参考的 Zed 组件

| 功能 | Zed 参考组件 |
|------|-------------|
| 窗口布局 | `Pane` / `Panel` |
| 模态框 | `Modal` |
| 列表/虚拟滚动 | `List` / `UniformList` |
| 表格 | `project search panel` |
| 树形结构 | `Outline` / `FileTree` |
| 搜索框 | `SearchBox` |
| 按钮/输入框 | `Button` / `TextField` |
| 上下文菜单 | `ContextMenu` |
| 通知 | `Notification` |

---

## 决策记录

### 需要用户确认的决策

1. **GPUI 来源**: 
   - [ ] 使用 Zed 源码中的 gpui (推荐)
   - [ ] 等 gpui 正式发布到 crates.io
   
2. **后端集成方式**:
   - [ ] 保持 HTTP API 调用 (推荐)
   - [ ] 直接调用后端 Rust 函数 (单二进制文件)

3. **移动端支持**:
   - [ ] 暂不支持 (GPUI 仅支持桌面)
   - [ ] 未来考虑 Web 版本

4. **功能取舍**:
   - [ ] 全功能迁移
   - [ ] 先迁移核心功能，高级功能后续迭代

---

## 附录：文件结构规划

```
gpui-app/
├── Cargo.toml
├── src/
│   ├── main.rs                 # 入口
│   ├── app.rs                  # App 定义
│   ├── lib.rs                  # 库导出
│   │
│   ├── ui/                     # UI 组件
│   │   ├── mod.rs
│   │   ├── views/
│   │   │   ├── mod.rs
│   │   │   ├── clusters_view.rs
│   │   │   ├── topics_view.rs
│   │   │   ├── messages_view.rs
│   │   │   ├── consumer_groups_view.rs
│   │   │   ├── schema_registry_view.rs
│   │   │   ├── settings_view.rs
│   │   │   ├── favorites_view.rs
│   │   │   └── topic_consumer_groups_view.rs
│   │   │
│   │   ├── components/
│   │   │   ├── mod.rs
│   │   │   ├── cluster_tree_navigator.rs
│   │   │   ├── topic_navigator.rs
│   │   │   ├── favorite_button.rs
│   │   │   ├── json_editor.rs
│   │   │   ├── message_query_tool.rs
│   │   │   ├── send_message_modal.rs
│   │   │   ├── sent_message_history.rs
│   │   │   ├── topic_history.rs
│   │   │   ├── create_topic_dialog.rs
│   │   │   ├── delete_topic_dialog.rs
│   │   │   ├── tour_overlay.rs
│   │   │   ├── context_menus/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── cluster_context_menu.rs
│   │   │   │   ├── topic_context_menu.rs
│   │   │   │   ├── partition_context_menu.rs
│   │   │   │   └── topics_folder_context_menu.rs
│   │   │   ├── settings/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── language_selector.rs
│   │   │   │   ├── json_highlight_selector.rs
│   │   │   └── layout/
│   │   │       ├── mod.rs
│   │   │       ├── left_sidebar.rs
│   │   │       ├── top_nav_bar.rs
│   │   │       ├── mobile_search_drawer.rs
│   │   │       ├── toast_and_confirm.rs
│   │   │       ├── context_menus.rs
│   │   │       └── modern_layout.rs
│   │   │
│   │   ├── theme/
│   │   │   ├── mod.rs
│   │   │   ├── colors.rs
│   │   │   └── styles.rs
│   │   │
│   │   └── elements/
│   │       ├── mod.rs
│   │       ├── card.rs          # Glass + Gradient border
│   │       ├── badge.rs
│   │       ├── button.rs
│   │       ├── input.rs
│   │       ├── select.rs
│   │       ├── table.rs
│   │       ├── modal.rs
│   │       └── toast.rs
│   │
│   ├── state/                  # 应用状态
│   │   ├── mod.rs
│   │   ├── app_state.rs
│   │   ├── cluster_state.rs
│   │   ├── theme_state.rs
│   │   ├── language_state.rs
│   │   └── update_state.rs
│   │
│   ├── api/                    # API 客户端
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── types.rs            # API 类型定义
│   │   ├── sse.rs              # SSE 流
│   │   └── error.rs
│   │
│   ├── i18n/                   # 国际化
│   │   ├── mod.rs
│   │   ├── zh.rs
│   │   ├── en.rs
│   │   └── translations.rs
│   │
│   ├── router/                 # 路由/视图切换
│   │   ├── mod.rs
│   │   └── router.rs
│   │
│   ├── utils/                  # 工具函数
│   │   ├── mod.rs
│   │   ├── json.rs
│   │   ├── json_highlight.rs
│   │   ├── time.rs
│   │   └── format.rs
│   │
│   └── tour/                   # 新手引导
│       ├── mod.rs
│       └── definitions.rs
│
├── icons/                      # 图标资源
│   └── icons.rs
│
├── build.rs                    # 构建脚本
└── assets/                     # 静态资源
```

---

## 结语

迁移到 GPUI 是一个大型工程，需要约 3-4 个月的开发时间。主要挑战是 GPUI 尚未正式发布，缺乏文档和成熟的组件库。

建议：
1. 先搭建最小可用原型，验证 GPUI 可行性
2. 参考 Zed 源码学习 GPUI API
3. 分阶段迁移，每完成一个模块就测试验证
4. 保持后端 API 不变，降低迁移风险