# Kafka Manager UI

一个现代化的 Kafka 多集群管理界面，使用 Vue 3、Tailwind CSS 4 和 DaisyUI 5 构建。

## 技术栈

- **Vue 3** - 渐进式 JavaScript 框架
- **TypeScript** - 类型安全的 JavaScript 超集
- **Tailwind CSS 4** - 实用优先的 CSS 框架
- **DaisyUI 5** - 基于 Tailwind CSS 的组件库
- **Vue Router** - 官方路由管理器
- **Pinia** - Vue 3 状态管理库

## 功能特性

### 多集群管理
- **侧边栏集群列表** - 同时显示所有集群，支持复选多选
- **集群健康状态** - 实时显示每个集群的连接状态（绿色=正常，红色=断开）
- **一键刷新** - 顶部导航栏刷新所有集群的健康状态
- **快速切换** - 点击复选框快速切换查看的集群

### Dashboard
- **全局统计** - 显示所有集群的汇总数据（总 Topic 数、分区数、Consumer Group 数）
- **集群状态卡片** - 每个集群显示连接状态、Topic 数、分区数、总 Lag
- **批量选择** - 支持全选/取消全选所有集群

### Topics 管理
- **按集群查看** - 分组显示每个集群的 Topics
- **统一查看** - 跨集群显示所有 Topics（带集群标签）
- **批量操作** - 支持创建、删除 Topic
- **分区详情** - 查看 Topic 的分区分布、Leader、ISR

### Consumer Groups 管理
- **多集群浏览** - 同时查看多个集群的 Consumer Groups
- **状态显示** - 显示 Group 状态（Stable/Empty/Dead 等）
- **Lag 监控** - 显示每个 Group 的总 Lag
- **Offset 重置** - 支持重置 Consumer Group Offset

### 告警规则
- **创建和管理** - 创建基于 Lag、速率的告警规则
- **多集群支持** - 为不同集群配置告警

## 快速开始

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run dev
```

开发服务器将在 http://localhost:5173 启动

### 构建生产版本

```bash
npm run build
```

### 预览生产构建

```bash
npm run preview
```

## 配置

### 环境变量

复制 `.env.example` 文件为 `.env.local` 进行配置：

```bash
cp .env.example .env.local
```

可用配置项：
- `VITE_API_URL` - 后端 API 地址
- `VITE_AUTH_ENABLED` - 是否启用认证
- `VITE_API_KEY` - API 密钥

### API 代理

开发环境下，API 请求通过 Vite 代理到后端服务。默认配置：

- 前端：http://localhost:5173
- 后端 API: http://localhost:8000

如需修改后端地址，编辑 `vite.config.ts`：

```typescript
server: {
  port: 5173,
  proxy: {
    '/api': {
      target: 'http://localhost:8000',  // 修改这里
      changeOrigin: true,
    },
  },
},
```

### 主题

默认使用 DaisyUI 的 `corporate` 主题。如需切换主题，编辑 `index.html`：

```html
<html lang="en" data-theme="corporate">
```

可用主题：light, dark, cupcake, bumblebee, corporate, retro, cyberpunk, valentine, halloween, aqua, lofi, luxury, dracula, night, coffee, winter, dim, nord, sunset

## 项目结构

```
ui/
├── src/
│   ├── api/           # API 客户端
│   │   └── client.ts
│   ├── components/    # 可复用组件
│   ├── layouts/       # 布局组件
│   │   └── MainLayout.vue    # 主布局（侧边栏 + 导航）
│   ├── router/        # 路由配置
│   │   └── index.ts
│   ├── stores/        # Pinia 状态管理
│   │   └── cluster.ts        # 集群状态（支持多选）
│   ├── types/         # TypeScript 类型定义
│   │   └── api.ts
│   └── views/         # 页面组件
│       ├── DashboardView.vue         # 多集群 Dashboard
│       ├── ClustersView.vue          # 集群管理
│       ├── TopicsView.vue            # Topics 管理（支持多集群）
│       ├── ConsumerGroupsView.vue    # Consumer Groups（支持多集群）
│       └── AlertsView.vue            # 告警规则
├── index.html
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## 多集群使用指南

### 1. 添加集群

1. 前往 **Clusters** 页面
2. 点击 **Add Cluster**
3. 填写集群名称和 Broker 地址

### 2. 选择集群查看

在左侧边栏：
- 勾选集群复选框选择一个或多个集群
- 点击 **All** 按钮选择所有集群
- 点击 **None** 按钮取消选择

### 3. 查看集群状态

- **绿色圆点** - 集群连接正常
- **红色圆点** - 集群连接失败
- 点击侧边栏的刷新按钮更新所有集群状态

### 4. 跨集群操作

在 **Topics** 和 **Consumer Groups** 页面：
- **By Cluster** - 按集群分组显示
- **All Topics/Groups** - 统一显示所有集群的数据

## API 端点

本 UI 与后端 API 配合使用，需要以下端点：

| 端点 | 方法 | 描述 |
|------|------|------|
| /api/health | GET | 健康检查 |
| /api/clusters | GET/POST | 获取/创建集群 |
| /api/clusters/:id | GET/PUT/DELETE | 集群操作 |
| /api/clusters/:id/test | POST | 测试集群连接 |
| /api/clusters/:id/stats | GET | 获取集群统计 |
| /api/clusters/:clusterId/topics | GET/POST | Topic 列表/创建 |
| /api/clusters/:clusterId/topics/:name | GET/DELETE | Topic 操作 |
| /api/clusters/:clusterId/topics/:name/offsets | GET | 获取 Topic Offset |
| /api/clusters/:clusterId/consumer-groups | GET | Consumer Group 列表 |
| /api/clusters/:clusterId/consumer-groups/:name | GET/DELETE | Consumer Group 操作 |
| /api/alert-rules | GET/POST | 告警规则管理 |

## 许可证

MIT
