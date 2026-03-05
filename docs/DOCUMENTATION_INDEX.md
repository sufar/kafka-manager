# 文档索引

本文档列出项目中所有文档文件及其用途。

## 主要文档

| 文档 | 描述 | 最后更新 |
|------|------|----------|
| [README.md](../README.md) | 项目主文档，包含功能列表、API 端点、项目结构 | 2025-03 |
| [API.md](../API.md) | API 端点详细说明 | 2025-03 |
| [WINDOWS_BUILD_GUIDE.md](../WINDOWS_BUILD_GUIDE.md) | Windows 打包指南 | 2025-03 |
| [TAURI_README.md](../TAURI_README.md) | Tauri 桌面应用说明 | 2025-03 |
| [TOPIC_REFRESH_FEATURE.md](../TOPIC_REFRESH_FEATURE.md) | Topic 刷新功能说明 | 2025-03 |
| [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) | 功能实现总结 | 2025-03 |

## 详细文档目录

| 文档 | 描述 |
|------|------|
| [API_EXAMPLES.md](API_EXAMPLES.md) | API 使用示例 |
| [OPTIMIZATIONS.md](OPTIMIZATIONS.md) | 性能优化记录 |
| [cluster_connection_api.md](cluster_connection_api.md) | 集群连接管理 API 文档 |

## 最近更新 (2025-03)

### Windows 打包修复
- 修复了 Windows 打包后后端启动失败的问题
- 增加了详细的诊断日志
- 改进了数据库路径处理（跨平台兼容）

### API 响应格式统一
- 所有 API 响应统一为 `{success: true, data: ...}` 或 `{success: false, error: ...}` 格式
- 前端和后端接口参数统一为 `cluster_id`

### 性能优化
1. **缓存优化**
   - Topic 元数据缓存: 5秒 → 30秒
   - Topic 列表缓存: 3秒 → 30秒
   - Consumer Group 缓存: 3秒 → 30秒
   - Broker 信息缓存: 10秒 → 60秒

2. **阻塞调用优化**
   - 将所有同步 Kafka 操作包装在 `tokio::task::spawn_blocking` 中
   - 避免阻塞 tokio 异步运行时

3. **数据库查询优化**
   - Topic 搜索从 N 次查询改为单次查询
   - 添加 `TopicStore::list_all()` 和 `TopicStore::search()` 方法

### 字段名统一
- Consumer Lag 接口统一使用 `name` 字段（替代 `group_id`/`group_name`）
- 所有接口统一使用 `cluster_id` 作为参数名

### 方法名修正
- `cluster_connection.*` 改为 `connection.*` 前缀

## 待办事项

- [ ] 更新 API_EXAMPLES.md 中的示例
- [ ] 补充性能测试报告
- [ ] 添加故障排除指南
