# 文档索引

## 核心文档

| 文档 | 说明 |
|------|------|
| [README.md](./README.md) | 项目概述、快速开始 |
| [API.md](./API.md) | 统一 API 接口文档 (POST /api + X-API-Method) |
| [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) | 技术实现总结 |

## 其他文档

| 文档 | 说明 |
|------|------|
| [TAURI_README.md](./TAURI_README.md) | Tauri 桌面应用开发指南 |
| [TOPIC_REFRESH_FEATURE.md](./TOPIC_REFRESH_FEATURE.md) | Topic 刷新功能说明 |
| [WINDOWS_BUILD_GUIDE.md](./WINDOWS_BUILD_GUIDE.md) | Windows 构建指南 |

## 变更日志

### 2026-03-07
- 清理所有 REST 风格 API 文档
- 统一为 POST /api + X-API-Method 风格
- 移除不必要的 console.log 和 unwrap()

### 2026-03-05
- Windows 打包修复
- API 格式统一为 `{success, data/error}`
- 性能优化（缓存、异步化、查询优化）

### 2026-02-26
- Topic 元数据同步功能
- `topic_metadata` 表
- `POST /api/clusters/:cluster_id/topics/refresh` API
