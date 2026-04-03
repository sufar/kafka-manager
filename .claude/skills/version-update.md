# 版本号更新指南

## 概述

本文档描述如何更新 Kafka Manager 应用的版本号。版本号需要同时在多个文件中保持一致。

## 版本号文件位置

| 文件 | 用途 | 格式 |
|------|------|------|
| `Cargo.toml` | 主项目 Rust 包版本 | `version = "x.y.z"` |
| `src-tauri/Cargo.toml` | Tauri 子项目版本 | `version = "x.y.z"` |
| `src-tauri/tauri.conf.json` | Tauri 应用版本（用于构建和更新） | `"version": "x.y.z"` |
| `ui/src/views/SettingsView.vue` | 前端显示的版本号 | `const appVersion = ref('x.y.z')` |

## 更新步骤

### 1. 更新 Rust 项目版本

**Cargo.toml**（根目录）
```toml
[package]
name = "kafka-manager-api"
version = "1.0.15"  # 修改此处
edition = "2021"
```

**src-tauri/Cargo.toml**
```toml
[package]
name = "kafka-manager"
version = "1.0.15"  # 修改此处
description = "A Kafka Manager tool built with Tauri 2"
```

### 2. 更新 Tauri 配置

**src-tauri/tauri.conf.json**
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Kafka Manager",
  "version": "1.0.15",  # 修改此处
  "identifier": "com.kafka-manager"
}
```

### 3. 更新前端显示版本

**ui/src/views/SettingsView.vue**

两处需要修改：

```typescript
// 第 345 行附近 - 默认版本号
const appVersion = ref('1.0.15');

// 第 394 行附近 - fallback 版本号
appVersion.value = typeof result === 'string' ? result : (result.version || '1.0.15');
```

### 4. 验证修改

运行以下命令确认所有版本号已更新：

```bash
grep -r "1\.0\.15" Cargo.toml src-tauri/Cargo.toml src-tauri/tauri.conf.json ui/src/views/SettingsView.vue
```

### 5. 更新 Cargo.lock（可选）

```bash
cargo build
```

这会更新 `Cargo.lock` 文件中的版本信息。

## 版本号命名规范

本项目使用语义化版本（Semantic Versioning）：

- **主版本号（Major）**：不兼容的 API 变更
- **次版本号（Minor）**：向后兼容的功能增强
- **修订号（Patch）**：向后兼容的问题修复

当前版本格式：`1.0.x`

## 发布检查清单

更新版本号后，确认以下内容：

- [ ] 所有 4 个文件的版本号一致
- [ ] `Cargo.lock` 已更新（运行 `cargo build`）
- [ ] 设置页面的版本号显示正确
- [ ] Git commit 信息包含版本号（例如：`升级 v1.0.15`）

## 相关文件

- `/Cargo.toml` - 主项目配置
- `/src-tauri/Cargo.toml` - Tauri 子项目配置
- `/src-tauri/tauri.conf.json` - Tauri 应用配置
- `/ui/src/views/SettingsView.vue` - 设置页面（显示版本号）

## 注意事项

1. **版本号必须同步**：所有 4 个文件的版本号必须保持一致，否则可能导致构建或运行时问题
2. **Tauri  updater**：`tauri.conf.json` 中的版本号用于 Tauri 的自动更新功能，必须正确设置
3. **前端 fallback**：`SettingsView.vue` 中的 fallback 版本号确保在无法获取实际版本时显示正确的默认值
4. **Git 提交**：每次版本更新都应该有对应的 commit，commit message 格式：`升级 vX.Y.Z`
