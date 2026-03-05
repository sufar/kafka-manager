# 文档索引

## 核心文档

| 文档 | 说明 |
|------|------|
| [README.md](./README.md) | 项目概述、快速开始 |
| [API.md](./API.md) | 统一风格 API 文档 (POST /api + X-API-Method) |
| [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) | 技术实现总结 |

## 详细文档

| 文档 | 说明 |
|------|------|
| [docs/API_EXAMPLES.md](./docs/API_EXAMPLES.md) | API 使用示例 |
| [docs/OPTIMIZATIONS.md](./docs/OPTIMIZATIONS.md) | 性能优化记录 |

## 变更日志

### 2026-03-05
- Windows 打包修复
- API 格式统一为 `{success, data/error}`
- 性能优化（缓存、异步化、查询优化）

### 2026-02-26
- Topic 元数据同步功能
- `topic_metadata` 表
- `POST /api/clusters/:cluster_id/topics/refresh` API
