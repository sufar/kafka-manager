# 集群连接管理 API 文档

## 概述

集群连接管理 API 用于管理 Kafka 集群的运行时连接状态，支持断开、重连、状态查询和健康检查功能。这些操作**不会影响数据库中的集群配置**，仅影响运行时连接。

## 基础信息

- **Base URL**: `http://<host>:<port>/api`
- **认证**: 需要在请求头中携带 API Key（如果启用了认证）
  ```
  Authorization: Bearer <your-api-key>
  ```

## API 端点

### 核心功能

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/cluster-connections` | GET | 获取所有集群连接状态 |
| `/api/cluster-connections/:id/status` | GET | 获取指定集群状态 |
| `/api/cluster-connections/:id/disconnect` | POST | 断开集群连接 |
| `/api/cluster-connections/:id/reconnect` | POST | 重连集群 |
| `/api/cluster-connections/:id/health-check` | POST | 健康检查 |

### 监控与历史

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/cluster-connections/:id/metrics` | GET | 获取连接池指标 |
| `/api/cluster-connections/:id/history` | GET | 获取连接历史 |
| `/api/cluster-connections/:id/stats` | GET | 获取连接统计 |

### 批量操作

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/cluster-connections/batch/disconnect` | POST | 批量断开集群 |
| `/api/cluster-connections/batch/reconnect` | POST | 批量重连集群 |

---

## API 详解

### 1. 获取所有集群连接状态

**请求**
```http
GET /api/cluster-connections
```

**响应示例**
```json
{
  "connections": [
    {"cluster_id": "prod-cluster", "status": "connected", "error_message": null},
    {"cluster_id": "dev-cluster", "status": "error", "error_message": "Connection refused"}
  ]
}
```

### 2. 获取指定集群状态

**请求**
```http
GET /api/cluster-connections/:id/status
```

**响应示例**
```json
{
  "cluster_id": "prod-cluster",
  "status": "connected",
  "error_message": null
}
```

### 3. 断开集群连接

**请求**
```http
POST /api/cluster-connections/:id/disconnect
```

**响应示例**
```json
{
  "success": true,
  "message": "Cluster 'prod-cluster' disconnected successfully"
}
```

### 4. 重连集群

**请求**
```http
POST /api/cluster-connections/:id/reconnect
```

**响应示例**
```json
{
  "success": true,
  "message": "Cluster 'prod-cluster' reconnected successfully"
}
```

### 5. 健康检查

**请求**
```http
POST /api/cluster-connections/:id/health-check
```

**响应示例**
```json
{
  "cluster_id": "prod-cluster",
  "healthy": true,
  "status": "healthy",
  "error_message": null
}
```

### 6. 获取连接池指标

**请求**
```http
GET /api/cluster-connections/:id/metrics
```

**响应示例**
```json
{
  "cluster_id": "prod-cluster",
  "consumer_pool_size": 10,
  "producer_pool_size": 10,
  "consumer_pool_available": 8,
  "producer_pool_available": 9
}
```

### 7. 获取连接历史

**请求**
```http
GET /api/cluster-connections/:id/history?limit=100
```

**响应示例**
```json
{
  "cluster_id": "prod-cluster",
  "history": [
    {
      "status": "connected",
      "error_message": null,
      "latency_ms": 15,
      "checked_at": "2026-02-26T10:30:00Z"
    }
  ]
}
```

### 8. 获取连接统计

**请求**
```http
GET /api/cluster-connections/:id/stats
```

**响应示例**
```json
{
  "cluster_id": "prod-cluster",
  "total_checks": 2880,
  "successful_checks": 2875,
  "failed_checks": 5,
  "success_rate": 99.83,
  "avg_latency_ms": 18.5,
  "last_status": "connected",
  "last_checked_at": "2026-02-26T10:30:00Z"
}
```

### 9. 批量断开集群

**请求**
```http
POST /api/cluster-connections/batch/disconnect
Content-Type: application/json

{"cluster_names": ["dev-cluster", "staging-cluster"]}
```

**响应示例**
```json
{
  "total": 2,
  "successful": 2,
  "failed": 0,
  "results": [
    {"cluster_name": "dev-cluster", "success": true, "message": "Disconnected successfully"}
  ]
}
```

### 10. 批量重连集群

**请求**
```http
POST /api/cluster-connections/batch/reconnect
Content-Type: application/json

{"cluster_names": ["dev-cluster", "staging-cluster"]}
```

**响应示例**
```json
{
  "total": 2,
  "successful": 1,
  "failed": 1,
  "results": [
    {"cluster_name": "dev-cluster", "success": true, "message": "Reconnected successfully"},
    {"cluster_name": "staging-cluster", "success": false, "message": "Cluster not found in database"}
  ]
}
```

---

## 配置选项

以下环境变量可用于配置健康检查后台任务：

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `HEALTH_CHECK_INTERVAL_SECS` | 30 | 健康检查间隔（秒） |
| `HEALTH_CHECK_RETENTION_HOURS` | 24 | 历史数据保留时间（小时） |
| `HEALTH_CHECK_AUTO_RECONNECT` | false | 是否启用自动重连 |
| `HEALTH_CHECK_MAX_RECONNECT_RETRIES` | 3 | 自动重连最大重试次数 |

**示例**：
```bash
export HEALTH_CHECK_INTERVAL_SECS=60
export HEALTH_CHECK_RETENTION_HOURS=48
export HEALTH_CHECK_AUTO_RECONNECT=true
```

---

## 自动健康检查

系统启动后会自动启动后台健康检查任务：

- **检查频率**: 默认每 30 秒检查所有集群
- **历史记录**: 自动记录每次检查结果到数据库
- **自动清理**: 每小时清理一次过期历史数据（保留 24 小时）
- **自动重连**: 如果启用，会在检测到错误时自动尝试重连（最多 3 次）

---

## 使用场景

### 场景 1：维护期间断开集群

```bash
# 断开集群
curl -X POST http://localhost:8080/api/cluster-connections/maintenance-cluster/disconnect

# 确认状态
curl http://localhost:8080/api/cluster-connections/maintenance-cluster/status
```

### 场景 2：维护后重连集群

```bash
# 重连集群
curl -X POST http://localhost:8080/api/cluster-connections/maintenance-cluster/reconnect

# 健康检查确认
curl -X POST http://localhost:8080/api/cluster-connections/maintenance-cluster/health-check
```

### 场景 3：监控所有集群状态

```bash
# 获取所有集群状态
curl http://localhost:8080/api/cluster-connections

# 获取集群统计
curl http://localhost:8080/api/cluster-connections/prod-cluster/stats

# 获取连接历史（用于分析趋势）
curl "http://localhost:8080/api/cluster-connections/prod-cluster/history?limit=1000"
```

### 场景 4：批量维护

```bash
# 断开多个集群
curl -X POST http://localhost:8080/api/cluster-connections/batch/disconnect \
  -H "Content-Type: application/json" \
  -d '{"cluster_names": ["dev-1", "dev-2", "dev-3"]}'

# 维护完成后批量重连
curl -X POST http://localhost:8080/api/cluster-connections/batch/reconnect \
  -H "Content-Type: application/json" \
  -d '{"cluster_names": ["dev-1", "dev-2", "dev-3"]}'
```

### 场景 5：诊断连接问题

```bash
# 检查集群状态
curl http://localhost:8080/api/cluster-connections/prod-cluster/status

# 查看连接池指标
curl http://localhost:8080/api/cluster-connections/prod-cluster/metrics

# 查看详细历史
curl "http://localhost:8080/api/cluster-connections/prod-cluster/history?limit=50"
```

---

## 错误码

| HTTP 状态码 | 说明 |
|-------------|------|
| 200 OK | 请求成功 |
| 400 Bad Request | 请求参数错误 |
| 404 Not Found | 集群不存在 |
| 500 Internal Server Error | 服务器内部错误 |

---

## 常见问题

### Q: 断开连接和从数据库删除集群有什么区别？
- **断开连接**: 仅移除运行时连接，数据库配置保留，可随时重连
- **删除集群**: 从数据库永久删除配置，需要重新创建才能使用

### Q: 重连时使用的配置是什么？
重连时会从数据库读取最新的集群配置（brokers、timeout 等）。

### Q: 健康检查和状态查询有什么区别？
- **状态查询**: 返回当前记录的连接状态
- **健康检查**: 主动发起一次连接测试，返回实时检测结果

### Q: 自动重连如何工作？
当后台健康检查检测到集群连接错误时，如果启用了 `HEALTH_CHECK_AUTO_RECONNECT`，会自动尝试重连，最多重试 `HEALTH_CHECK_MAX_RECONNECT_RETRIES` 次。
