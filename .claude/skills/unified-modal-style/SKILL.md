---
name: unified-modal-style
description: "统一弹窗风格"
type: skill
---

# 统一弹窗风格

## 概述

本项目采用统一的现代弹窗（Modal）设计风格，具有以下特点：
- 渐变背景图标容器
- 紧凑的头部布局（标题 + 副标题）
- 网格化表单布局
- 图标标签增强可读性
- 可展开的高级选项（带动画）
- 无背景模糊效果

## 快速开始

### 方式一：使用 ModernModal 组件（推荐）

```vue
<template>
  <ModernModal
    v-model="showModal"
    title="弹窗标题"
    subtitle="副标题或描述"
    :icon-slot="true"
  >
    <!-- 图标插槽 -->
    <template #icon>
      <svg><!-- 你的图标 --></svg>
    </template>

    <!-- 内容插槽 -->
    <form @submit.prevent="handleSubmit" class="space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- 表单字段 -->
      </div>

      <!-- 底部按钮 -->
      <template #footer>
        <button type="button" class="btn btn-ghost btn-sm" @click="showModal = false">取消</button>
        <button type="submit" class="btn btn-primary btn-sm">提交</button>
      </template>
    </form>
  </ModernModal>
</template>

<script setup>
import { ref } from 'vue';
import ModernModal from '@/components/ModernModal.vue';

const showModal = ref(false);
</script>
```

### 方式二：手动实现完整结构

```vue
<Teleport to="body">
  <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" @click.self="closeModal">
    <div class="modal-box w-full max-w-2xl mx-2 md:mx-auto p-5">
      <!-- Header -->
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
            <svg><!-- 图标 --></svg>
          </div>
          <div>
            <h3 class="font-bold text-base">标题</h3>
            <span class="text-xs text-base-content/60 font-mono">副标题</span>
          </div>
        </div>
        <button class="btn btn-sm btn-circle btn-ghost" @click="closeModal">
          <svg><!-- 关闭图标 --></svg>
        </button>
      </div>

      <form @submit.prevent="handleSubmit" class="space-y-4">
        <!-- 表单内容 -->
      </form>
    </div>
  </dialog>
</Teleport>
```

## 核心样式规范

### 1. 容器样式

| 属性 | 值 | 说明 |
|------|-----|------|
| `max-width` | `max-w-2xl` | 最大宽度 672px |
| `margin` | `mx-2 md:mx-auto` | 移动端留边，桌面端居中 |
| `padding` | `p-5` | 内边距 1.25rem |
| `border-radius` | 继承 `modal-box` | DaisyUI 默认圆角 |

### 2. 头部图标容器

```html
<div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
  <svg class="w-4 h-4 text-primary"><!-- 图标 --></svg>
</div>
```

**关键样式：**
- 尺寸：`w-9 h-9` (36x36px)
- 圆角：`rounded-xl` (大圆角)
- 背景：`bg-gradient-to-br from-primary/20 to-secondary/20` (渐变半透明)
- 图标：`w-4 h-4 text-primary` (16x16px，主色)

### 3. 标题样式

```html
<h3 class="font-bold text-base">标题文本</h3>
<span class="text-xs text-base-content/60 font-mono">副标题文本</span>
```

**规范：**
- 主标题：`font-bold text-base` (加粗，1rem)
- 副标题：`text-xs text-base-content/60 font-mono` (12px，60% 透明度，等宽字体)

### 4. 表单字段布局

#### 单列布局
```html
<div>
  <label class="block text-sm font-medium mb-1.5">
    <span class="flex items-center gap-2">
      <svg class="w-4 h-4"><!-- 图标 --></svg>
      字段名 <span class="text-error text-xs ml-1">必填</span>
    </span>
  </label>
  <input class="input input-bordered input-sm w-full" />
</div>
```

#### 双列网格布局
```html
<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
  <!-- 字段 1 -->
  <div>
    <label class="block text-sm font-medium mb-1.5">
      <span class="flex items-center gap-2">
        <svg class="w-4 h-4"><!-- 图标 --></svg>
        字段名
      </span>
    </label>
    <input class="input input-bordered input-sm w-full" />
  </div>
  <!-- 字段 2 -->
</div>
```

### 5. 输入框样式

| 类型 | 样式类 | 说明 |
|------|--------|------|
| 文本输入 | `input input-bordered input-sm w-full` | 标准输入框 |
| 下拉选择 | `select select-bordered select-sm w-full` | 标准下拉框 |
| 多行文本 | `textarea textarea-bordered w-full` | 多行输入 |

### 6. 可展开区域

```html
<div class="border-t border-base-content/10 pt-4">
  <button type="button" class="btn btn-ghost btn-sm w-full justify-between group" @click="showAdvanced = !showAdvanced">
    <span class="flex items-center gap-2">
      <svg class="w-4 h-4 transition-transform group-hover:rotate-90" :class="{ 'rotate-90': showAdvanced }">
        <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
      </svg>
      高级选项
    </span>
    <span class="text-xs text-base-content/50">{{ showAdvanced ? '收起' : '展开' }}</span>
  </button>

  <div v-if="showAdvanced" class="mt-3 space-y-3 animate-fadeIn">
    <!-- 展开内容 -->
  </div>
</div>
```

### 7. 底部操作按钮

```html
<div class="modal-action flex-wrap gap-2 pt-3">
  <button type="button" class="btn btn-ghost btn-sm" @click="closeModal">取消</button>
  <button type="submit" class="btn btn-primary btn-sm flex items-center gap-2" :disabled="submitting">
    <span v-if="submitting" class="loading loading-spinner loading-sm"></span>
    提交
  </button>
</div>
```

## CSS 样式

### 必需的 CSS (添加到组件 `<style scoped>`)

```css
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}

/* 移除背景模糊效果 */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
```

## 图标使用规范

为每个表单字段选择合适的图标：

| 字段类型 | 推荐图标 | SVG 路径参考 |
|----------|----------|--------------|
| 名称/标题 | 文档/标题图标 | `M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z` |
| 地址/连接 | 服务器/集群图标 | `M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01` |
| 时间/超时 | 时钟图标 | `M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z` |
| 分区/网格 | 方格图标 | `M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6Z` |
| 分组 | 文件夹图标 | `M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44z` |

## 示例参考

### 已实现的弹窗

| 文件 | 路径 | 说明 |
|------|------|------|
| SendMessageModal.vue | `ui/src/components/SendMessageModal.vue` | 发送消息弹窗（参考标准） |
| ClustersView.vue | `ui/src/views/ClustersView.vue` | 添加/编辑集群弹窗 |
| TopicsView.vue | `ui/src/views/TopicsView.vue` | 创建 Topic 弹窗 |
| FavoriteButton.vue | `ui/src/components/FavoriteButton.vue` | 选择分组弹窗（原生 dialog） |
| ClusterTreeNavigator.vue | `ui/src/components/ClusterTreeNavigator.vue` | 连接错误弹窗（原生 dialog） |

## 迁移清单

需要改造的弹窗组件：

| 组件 | 当前状态 | 优先级 |
|------|----------|--------|
| SendMessageModal.vue | 已改造 (使用 ModernModal) | P1 |
| ClustersView.vue | 已改造 (使用 ModernModal) | P1 |
| TopicsView.vue | 已改造 (使用 ModernModal) | P1 |
| FavoriteButton.vue | 已改造 (使用原生 dialog) | P1 |
| ClusterTreeNavigator.vue | 已改造 (使用原生 dialog) | P1 |
| ConsumerGroupsView | 待改造 | P2 |
| ConsumerLagView | 待改造 | P2 |
| SettingsView | 待改造 | P2 |
| 其他自定义弹窗 | 待评估 | P3 |

## 方式三：使用原生 dialog（推荐用于简单弹窗）

对于简单的弹窗（如错误提示、确认对话框），可以直接使用原生 `<dialog>` 元素，无需 ModernModal 组件：

```vue
<template>
  <Teleport to="body">
    <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" @click.self="closeModal">
      <div class="modal-box w-full max-w-sm mx-2 md:mx-auto p-5">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-error/20 to-error/10 flex items-center justify-center">
              <svg class="w-4 h-4 text-error"><!-- 图标 --></svg>
            </div>
            <h3 class="font-bold text-base">标题</h3>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeModal">
            <svg><!-- 关闭图标 --></svg>
          </button>
        </div>

        <!-- Content -->
        <div class="space-y-3">
          <!-- 内容 -->
        </div>

        <!-- Footer -->
        <div class="modal-action flex-wrap gap-2 pt-3">
          <button type="button" class="btn btn-ghost btn-sm" @click="closeModal">取消</button>
          <button type="button" class="btn btn-error btn-sm" @click="handleRetry">重试</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </Teleport>
</template>

<script setup>
import { ref, nextTick } from 'vue';

const modalRef = ref<HTMLDialogElement | null>(null);

async function openModal() {
  await nextTick();
  modalRef.value?.showModal();
}

function closeModal() {
  modalRef.value?.close();
}
</script>

<style scoped>
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
</style>
```

**使用原生 dialog 的场景：**
- 简单的错误提示对话框
- 确认对话框
- 不需要复杂表单的弹窗

## 注意事项

1. **Teleport**: 始终使用 `<Teleport to="body">` 包裹弹窗，避免层级问题
2. **关闭方式**: 使用 `@click.self` 而非 `modal-backdrop` 表单
3. **响应式**: 使用 `modal-bottom sm:modal-middle` 实现移动端从底部弹出
4. **国际化**: 所有文本使用 `t.yourComponent.key` 格式
5. **必填标记**: 必填字段后添加 `<span class="text-error text-xs ml-1">必填</span>`
6. **加载状态**: 提交按钮需显示加载动画 `loading loading-spinner`
