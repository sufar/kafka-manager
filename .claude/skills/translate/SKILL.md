---
name: translate
description: "中英文翻译技能"
type: skill
---

# 中英文翻译技能

用于项目国际化（i18n）的中英文翻译技能，帮助快速添加或更新翻译条目。

## 翻译范围

- `ui/src/i18n/translations.ts` 文件中的翻译条目
- 前端界面文本的中英文互译

## 使用方法

### 方法一：直接翻译文本

```
/translate 需要翻译的中文或英文文本
```

示例：
```
/translate 集群详情
```

### 方法二：添加新的翻译条目

1. 确定翻译类别（messages、clusters、topics 等）
2. 提供英文 key 和对应中文/英文文本
3. 自动添加到 `translations.ts` 的中英文两个版本

示例请求：
```
添加翻译：
- key: clusterName
- 中文：集群名称
- 类别：clusters
```

### 方法三：补全缺失的翻译

当代码中使用了翻译 key 但翻译文件中缺失时：
```
补全翻译：messages.cluster
```

## 翻译风格

### 中文翻译要求
- 简洁明了，符合中文 UI 习惯
- 技术术语使用标准译法
- 保持短语简短，适合按钮、标签等 UI 元素

### 英文翻译要求
- 使用简洁的英文 UI 语言
- 首字母大写规则：标题、标签首字母大写
- 保持与现有翻译风格一致

## 常用术语对照表

| 英文 | 中文 |
|------|------|
| Cluster | 集群 |
| Topic | 主题 |
| Partition | 分区 |
| Offset | 偏移量 |
| Message | 消息 |
| Broker | 代理 |
| Consumer Group | 消费组 |
| Producer | 生产者 |
| Consumer | 消费者 |
| Timestamp | 时间戳 |
| Configuration | 配置 |
| Settings | 设置 |
| Dashboard | 仪表盘 |
| Favorites | 收藏 |

## 翻译文件结构

`ui/src/i18n/translations.ts` 包含两个语言版本：

```typescript
export const translations: Record<Language, Translation> = {
  zh: {
    messages: {
      title: '消息',
      // ...
    },
  },
  en: {
    messages: {
      title: 'Messages',
      // ...
    },
  },
};
```

## 前端使用翻译

在 Vue 组件中使用：

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useLanguageStore } from '@/stores/language';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
</script>

<template>
  <h1>{{ t.messages.title }}</h1>
  <button>{{ t.common.save }}</button>
</template>
```

在模板中直接使用：
```vue
{{ t.messages.sendMessage }}
{{ t.common.confirm }}
```

## 示例

**用户**：`/translate 消息已发送`
**助手**：`Message sent`

**用户**：`/translate Cluster Details`
**助手**：`集群详情`

**用户**：添加翻译 `messages.clusterName`，中文"集群名称"
**助手**：自动在 `translations.ts` 中添加中英文条目
