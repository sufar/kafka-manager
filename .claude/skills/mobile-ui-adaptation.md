# Kafka Manager UI 移动端适配方案

## 现状分析

### 已具备的基础
- `index.html` 已设置 viewport meta 标签 (`width=device-width, initial-scale=1.0`)
- DaisyUI 已提供响应式工具类
- 项目使用 Tailwind CSS，支持响应式前缀

### 当前问题
1. **响应式类使用极少**: 整个项目仅约 10 处使用响应式前缀 (`sm:`, `md:`, `lg:`)
2. **固定布局**: 侧边栏固定宽度，主内容区无响应式处理
3. **工具栏过宽**: 多行输入框和按钮在小屏幕上会溢出
4. **表格数据**: 宽表格在移动端需要横向滚动或转为卡片
5. **触摸体验**: 按钮和交互元素可能过小（部分仅 6px 内边距）

## 适配策略

### 1. 布局层级改造 (ModernLayout.vue)

**桌面端 (>= md/768px)**: 保持现有设计
- 固定侧边栏 (320px)
- 顶部导航栏
- 可拖拽调整侧边栏宽度

**移动端 (< md)**: 抽屉式布局
- 侧边栏变为抽屉，默认隐藏
- 添加汉堡菜单按钮触发抽屉
- 抽屉从左侧滑入，覆盖主内容
- 点击遮罩或滑动关闭抽屉
- 主内容区全宽显示

关键改动:
```vue
<!-- 移动端抽屉遮罩 -->
<div v-if="isMobile && sidebarOpen" class="fixed inset-0 bg-black/50 z-40" @click="sidebarOpen = false"></div>

<!-- 侧边栏 -->
<aside :class="[
  'fixed md:relative z-50 transition-transform',
  isMobile ? (sidebarOpen ? 'translate-x-0' : '-translate-x-full') : ''
]">

<!-- 主内容区 -->
<main class="flex-1 w-full md:w-auto">
```

### 2. 顶部导航简化 (ModernLayout.vue)

**桌面端**: 保持搜索框、语言切换、主题切换、设置按钮

**移动端**:
- 左侧: 汉堡菜单按钮 + Logo
- 中间: 仅搜索图标，点击展开搜索抽屉
- 右侧: 更多按钮（语言/主题/设置放入下拉菜单）

关键改动:
```vue
<!-- 移动端简化头部 -->
<header class="h-10 md:h-10">
  <!-- 移动端汉堡菜单 -->
  <button v-if="isMobile" @click="sidebarOpen = true" class="btn btn-ghost btn-sm md:hidden">
    <svg><!-- hamburger icon --></svg>
  </button>

  <!-- 移动端搜索图标 -->
  <button v-if="isMobile" @click="showSearchDrawer = true" class="md:hidden">
    <svg><!-- search icon --></svg>
  </button>
</header>
```

### 3. 工具栏响应式 (MessageQueryTool.vue, TopicsView.vue)

**问题**: 当前工具栏使用 `flex items-center gap-2`，在移动端会溢出

**解决方案**:
- 小屏幕时垂直堆叠 `flex-col md:flex-row`
- 次要操作折叠进"更多"下拉菜单
- 输入框宽度自适应 `w-full md:w-48`

关键改动:
```vue
<!-- MessageQueryTool.vue -->
<div class="toolbar flex flex-col md:flex-row items-stretch md:items-center gap-2 p-3">
  <!-- 主要操作始终显示 -->
  <select class="select select-sm w-full md:w-48">...</select>

  <!-- 移动端次要操作折叠 -->
  <div class="flex gap-2 md:hidden">
    <button @click="showMoreActions = true">更多</button>
  </div>
</div>
```

### 4. 表格转卡片 (MessagesClassicView.vue, TopicsView.vue)

**方案 A: 卡片列表 (推荐用于消息列表)**
- 移动端每行消息转为卡片
- 卡片内垂直排列字段
- 支持滑动操作

**方案 B: 横向滚动 (适用于数据表格)**
- 表格容器添加 `overflow-x-auto`
- 保持表头固定
- 提示用户可横向滑动

关键改动 (MessageQueryTool.vue 消息表格):
```vue
<!-- 桌面端表格 -->
<table class="hidden md:table">...</table>

<!-- 移动端卡片列表 -->
<div class="md:hidden space-y-2">
  <div v-for="msg in messages" class="card bg-base-100 p-3 shadow-sm">
    <div class="flex justify-between">
      <span class="text-xs text-base-content/60">Offset: {{ msg.offset }}</span>
      <span class="text-xs">{{ formatTime(msg.timestamp) }}</span>
    </div>
    <div class="mt-2 text-sm font-mono truncate">{{ msg.value }}</div>
  </div>
</div>
```

### 5. 触摸优化

**最小点击区域**: 44x44px
- DaisyUI `btn-sm` 按钮实际高度约 32px，需增大
- 移动端使用 `btn-md` 或自定义 `min-h-[44px]`

**触摸反馈**:
- 使用 `active:scale-95` 提供按压反馈
- 按钮添加 `touch-manipulation` 防止双击缩放

**手势支持**:
- 侧边栏支持滑动关闭
- 消息列表支持下拉刷新

### 6. 断点配置

使用 Tailwind 默认断点:
- `sm`: 640px
- `md`: 768px (主要断点，切换布局)
- `lg`: 1024px
- `xl`: 1280px

## 文件改造清单

| 文件 | 改造内容 | 优先级 |
|------|----------|--------|
| ModernLayout.vue | 侧边栏抽屉化、响应式头部、isMobile检测 | P0 |
| TopicNavigator.vue | 全屏抽屉模式、触摸优化 | P1 |
| MessageQueryTool.vue | 工具栏响应式、表格卡片化 | P1 |
| TopicsView.vue | 工具栏响应式、表格卡片化 | P1 |
| MessagesClassicView.vue | 工具栏响应式、表格卡片化 | P1 |
| MessagesView.vue | 模式切换器响应式 | P2 |
| DashboardView.vue | Stats卡片响应式(已完成stats-vertical) | P2 |
| ClustersView.vue | 表格响应式 | P2 |
| ConsumerGroupsView.vue | 表格响应式 | P2 |
| ConsumerLagView.vue | 表格响应式 | P2 |

## 技术实现

### isMobile 检测
使用 CSS 媒体查询配合 JS:

```typescript
// ModernLayout.vue
const isMobile = ref(false);

function checkMobile() {
  isMobile.value = window.innerWidth < 768; // md breakpoint
}

onMounted(() => {
  checkMobile();
  window.addEventListener('resize', checkMobile);
});
```

### CSS 变量/类封装
添加移动端工具类:

```css
/* 移动端隐藏 */
.mobile-hidden { @apply hidden md:block; }

/* 桌面端隐藏 */
.desktop-hidden { @apply block md:hidden; }
```

## 验收标准

1. **布局**: 在 iPhone SE (375px)、iPhone 14 Pro Max (430px)、iPad (768px) 上正常显示
2. **侧边栏**: 移动端可通过汉堡菜单打开/关闭
3. **表格**: 消息列表在移动端显示为卡片，可正常浏览
4. **触摸**: 所有可点击元素最小 44px 点击区域
5. **性能**: 无明显的布局抖动或重绘

## 风险提示

1. **侧边栏拖拽**: 移动端需禁用侧边栏宽度拖拽，避免与滑动手势冲突
2. **搜索下拉**: 移动端搜索建议改为全屏或模态框，避免被键盘遮挡
3. **Context Menu**: 长按菜单在移动端需适配触摸长按事件
