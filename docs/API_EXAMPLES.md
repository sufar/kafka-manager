# Kafka Manager API 示例

本文档提供所有 API 端点的完整使用示例。

## 基础信息

- **默认地址**: `http://localhost:3000`
- **Content-Type**: `application/json`

## 认证

### 启用认证

通过环境变量启用 API 认证：

```bash
export API_KEYS="key1,key2,key3"
export AUTH_ENABLED=true
cargo run
```

### 使用认证

在请求 Header 中添加 `X-API-Key`：

```bash
curl -H "X-API-Key: key1" http://localhost:3000/api/health
```

### 跳过认证的路径

以下路径不需要认证：
- `/api/health`
- `/api/ready`

## 审计日志

所有 API 请求都会自动记录审计日志，包括：
- 操作时间
- 操作类型（GET/POST/PUT/DELETE）
- 请求路径
- 集群 ID
- 资源名称
- API Key（脱敏）
- 响应状态码
- 请求耗时
- 客户端 IP

日志级别：
- 状态码 >= 500：WARN 级别
- 其他：INFO 级别

## 目录

1. [集群管理](#集群管理)
2. [Topic 管理](#topic-管理)
3. [消息管理](#消息管理)
4. [Consumer Group 管理](#consumer-group-管理)
5. [集群监控](#集群监控)
6. [健康检查](#健康检查)
7. [认证管理](#认证管理)
8. [审计日志](#审计日志)
9. [告警规则管理](#告警规则管理)

---

## 集群管理

### 创建 Kafka 集群

```bash
curl -X POST http://localhost:3000/api/clusters \
  -H "Content-Type: application/json" \
  -d '{
    "name": "development",
    "brokers": "localhost:9092",
    "request_timeout_ms": 5000,
    "operation_timeout_ms": 5000
  }'
```

**响应**:
```json
{
  "id": 1,
  "name": "development",
  "brokers": "localhost:9092",
  "request_timeout_ms": 5000,
  "operation_timeout_ms": 5000,
  "created_at": "2026-02-25T10:00:00Z",
  "updated_at": "2026-02-25T10:00:00Z"
}
```

### 列出所有集群

```bash
curl http://localhost:3000/api/clusters
```

**响应**:
```json
{
  "clusters": [
    {
      "id": 1,
      "name": "development",
      "brokers": "localhost:9092",
      "request_timeout_ms": 5000,
      "operation_timeout_ms": 5000,
      "created_at": "2026-02-25T10:00:00Z",
      "updated_at": "2026-02-25T10:00:00Z"
    }
  ]
}
```

### 获取集群详情

```bash
curl http://localhost:3000/api/clusters/1
```

**响应**:
```json
{
  "id": 1,
  "name": "development",
  "brokers": "localhost:9092",
  "request_timeout_ms": 5000,
  "operation_timeout_ms": 5000,
  "created_at": "2026-02-25T10:00:00Z",
  "updated_at": "2026-02-25T10:00:00Z"
}
```

### 更新集群

```bash
curl -X PUT http://localhost:3000/api/clusters/1 \
  -H "Content-Type: application/json" \
  -d '{
    "brokers": "localhost:9092,localhost:9093"
  }'
```

### 删除集群

```bash
curl -X DELETE http://localhost:3000/api/clusters/1
```

### 测试集群连接

```bash
curl -X POST http://localhost:3000/api/clusters/1/test
```

**响应**:
```json
{
  "success": true
}
```

---

## Topic 管理

### 创建 Topic

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-topic",
    "num_partitions": 3,
    "replication_factor": 1
  }'
```

**响应**:
```json
{
  "name": "test-topic"
}
```

### 列出所有 Topic

```bash
curl http://localhost:3000/api/clusters/development/topics
```

**响应**:
```json
{
  "topics": ["test-topic", "user-events", "order-logs"]
}
```

### 查看 Topic 详情

```bash
curl http://localhost:3000/api/clusters/development/topics/test-topic
```

**响应**:
```json
{
  "name": "test-topic",
  "partitions": [
    {
      "id": 0,
      "leader": 1,
      "replicas": [1],
      "isr": [1]
    },
    {
      "id": 1,
      "leader": 2,
      "replicas": [2],
      "isr": [2]
    },
    {
      "id": 2,
      "leader": 1,
      "replicas": [1],
      "isr": [1]
    }
  ]
}
```

### 删除 Topic

```bash
curl -X DELETE http://localhost:3000/api/clusters/development/topics/test-topic
```

### 增加分区数

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/test-topic/partitions \
  -H "Content-Type: application/json" \
  -d '{
    "new_partitions": 6
  }'
```

### 查看 Topic 配置

```bash
curl http://localhost:3000/api/clusters/development/topics/test-topic/config
```

**响应**:
```json
{
  "cleanup.policy": "delete",
  "retention.ms": "604800000",
  "segment.bytes": "1073741824",
  "min.insync.replicas": "1"
}
```

### 修改 Topic 配置

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/test-topic/config \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "retention.ms": "1209600000",
      "min.insync.replicas": "2"
    }
  }'
```

### 批量创建 Topic

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/batch \
  -H "Content-Type: application/json" \
  -d '{
    "topics": [
      {
        "name": "topic-1",
        "num_partitions": 3,
        "replication_factor": 1
      },
      {
        "name": "topic-2",
        "num_partitions": 6,
        "replication_factor": 1
      }
    ],
    "continue_on_error": true
  }'
```

**响应**:
```json
{
  "success": true,
  "created": ["topic-1", "topic-2"],
  "failed": []
}
```

### 批量删除 Topic

```bash
curl -X DELETE http://localhost:3000/api/clusters/development/topics/batch \
  -H "Content-Type: application/json" \
  -d '{
    "topics": ["topic-1", "topic-2"],
    "continue_on_error": true
  }'
```

**响应**:
```json
{
  "success": true,
  "deleted": ["topic-1", "topic-2"],
  "failed": []
}
```

### 查看 Topic 分区 Offset 信息

```bash
curl http://localhost:3000/api/clusters/development/topics/test-topic/offsets
```

**响应**:
```json
[
  {
    "topic": "test-topic",
    "partition": 0,
    "leader": 1,
    "replicas": [1],
    "isr": [1],
    "earliest_offset": 0,
    "latest_offset": 1000,
    "first_commit_time": null,
    "last_commit_time": null
  },
  {
    "topic": "test-topic",
    "partition": 1,
    "leader": 2,
    "replicas": [2],
    "isr": [2],
    "earliest_offset": 0,
    "latest_offset": 1500,
    "first_commit_time": null,
    "last_commit_time": null
  }
]
```

**字段说明**:
- `earliest_offset`: 分区最早可用 offset
- `latest_offset`: 分区最新 offset (log end offset)
- `first_commit_time`: 最早消息的时间戳 (毫秒，暂不支持)
- `last_commit_time`: 最新消息的时间戳 (毫秒，暂不支持)

### 查看 Topic 生产速度

```bash
curl http://localhost:3000/api/clusters/development/topics/test-topic/throughput
```

**响应**:
```json
{
  "topic": "test-topic",
  "produce_throughput": {
    "messages_per_second": 150.5,
    "bytes_per_second": null,
    "window_seconds": 3600
  },
  "total_messages": 541800,
  "partitions": [
    {
      "partition": 0,
      "earliest_offset": 0,
      "latest_offset": 180600,
      "message_count": 180600,
      "produce_rate": 50.2,
      "first_message_time": 1708876800000,
      "last_message_time": 1708880400000
    },
    {
      "partition": 1,
      "earliest_offset": 0,
      "latest_offset": 180600,
      "message_count": 180600,
      "produce_rate": 50.1,
      "first_message_time": 1708876800000,
      "last_message_time": 1708880400000
    }
  ]
}
```

**字段说明**:
- `messages_per_second`: 生产速率（条/秒）
- `window_seconds`: 统计时间窗口（秒）
- `total_messages`: Topic 总消息数
- `produce_rate`: 每个分区的生产速率
- `first_message_time`: 最早消息时间戳（毫秒）
- `last_message_time`: 最新消息时间戳（毫秒）

### 导出 Topic 消息

```bash
# 导出为 JSON
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages/export?format=json&max_messages=1000" \
  -o messages.json

# 导出为 CSV
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages/export?format=csv&max_messages=1000" \
  -o messages.csv

# 按时间范围导出
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages/export?format=json&start_time=1708876800000&end_time=1708880400000" \
  -o messages.json
```

**查询参数**:
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| format | string | json | 导出格式：json, csv, text |
| max_messages | int | 1000 | 最大导出消息数 |
| start_time | i64 | 无 | 开始时间戳（毫秒） |
| end_time | i64 | 无 | 结束时间戳（毫秒） |
| filename | string | 自动生成 | 自定义文件名 |

---

## 消息管理

### 发送消息

```bash
# 不指定分区（由 Kafka 根据 key 决定）
curl -X POST http://localhost:3000/api/clusters/development/topics/test-topic/messages \
  -H "Content-Type: application/json" \
  -d '{
    "key": "key1",
    "value": "Hello, Kafka!"
  }'
```

### 发送到指定分区

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/test-topic/messages \
  -H "Content-Type: application/json" \
  -d '{
    "key": "user-123",
    "value": "Hello to partition 2!",
    "partition": 2
  }'
```

**响应**:
```json
{
  "partition": 2,
  "offset": 42
}
```

### 查看消息

```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?partition=0&max_messages=100"
```

**查询参数**:

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| partition | int | 无 | 指定分区 ID |
| offset | int | 无 | 最小 offset，过滤掉之前的消息 |
| max_messages | int | 100 | 最大获取消息数（从 Kafka 读取的上限） |
| order_by | string | 无 | 排序字段："timestamp" 或 "offset" |
| sort | string | "asc" | 排序方向："asc"（升序）或 "desc"（降序） |
| limit | int | 无 | 限制返回的消息数量（排序后截断） |
| search | string | 无 | 搜索关键词（模糊匹配） |
| search_in | string | "all" | 搜索范围："key", "value", "all" |
| format | string | "raw" | 输出格式："raw", "json", "hex" |
| decode | string | "utf8" | 解码方式："utf8", "base64", "hex" |

### 增强消息查看（推荐）

```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?format=json&decode=base64"
```

**增强响应**:
```json
{
  "messages": [
    {
      "partition": 0,
      "offset": 100,
      "key": "user-123",
      "value": "{\"id\": 1, \"name\": \"test\"}",
      "timestamp": 1708876800000,
      "key_raw": "dXNlci0xMjM=",
      "value_raw": "eyJpZCI6IDEsICJuYW1lIjogInRlc3QifQ==",
      "value_json": {"id": 1, "name": "test"},
      "value_hex": "7b 22 69 64 22 3a 20 31 ...",
      "content_type": "application/json",
      "size": {
        "key_size": 8,
        "value_size": 25,
        "total_size": 33
      }
    }
  ],
  "count": 1,
  "format": "json",
  "decode_mode": "base64"
}
```

**字段说明**:
- `key_raw`: Base64 编码的原始 key
- `value_raw`: Base64 编码的原始 value
- `value_json`: JSON 格式化后的 value（如果可解析）
- `value_hex`: Hex 格式的 value
- `content_type`: 推断的内容类型
- `size`: 消息大小信息

**示例 - 按时间戳降序排序**:
```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?order_by=timestamp&sort=desc&limit=10"
```

**示例 - 按 offset 升序排序**:
```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?order_by=offset&sort=asc&limit=50"
```

**示例 - 搜索 key 包含 "user-123" 的消息**:
```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?search=user-123&search_in=key"
```

**示例 - 搜索 value 包含 "error" 的消息**:
```bash
curl "http://localhost:3000/api/clusters/development/topics/test-topic/messages?search=error&search_in=value"
```

**响应**:
```json
{
  "messages": [
    {
      "partition": 0,
      "offset": 100,
      "key": "user-123",
      "value": "Hello, World!",
      "timestamp": 1708876800000
    },
    {
      "partition": 0,
      "offset": 101,
      "key": "user-456",
      "value": "Test message",
      "timestamp": 1708876801000
    }
  ]
}
```

---

## Consumer Group 管理

### 列出 Consumer Groups

```bash
curl http://localhost:3000/api/clusters/development/consumer-groups
```

**响应**:
```json
{
  "groups": []
}
```

**注意**: 由于 rdkafka 0.39 限制，ListGroups API 未暴露，返回空列表。建议通过 offsets API 查看有消费进度的 group。

### 查看 Consumer Group 详情

```bash
curl http://localhost:3000/api/clusters/development/consumer-groups/my-group
```

**响应**:
```json
{
  "name": "my-group",
  "state": "Unknown",
  "protocol": "Unknown",
  "members": [],
  "offsets": []
}
```

**注意**: 由于 rdkafka 0.39 限制，无法获取实际的 Group 状态和成员信息。

### 删除 Consumer Group

```bash
curl -X DELETE http://localhost:3000/api/clusters/development/consumer-groups/my-group
```

### 查看 Consumer Group Offset 和 Lag

```bash
curl "http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets?topic=test-topic"
```

**响应**:
```json
{
  "group_name": "my-group",
  "topic": "test-topic",
  "partitions": [
    {
      "partition": 0,
      "current_offset": 800,
      "log_end_offset": 1000,
      "lag": 200,
      "state": "Active",
      "last_commit_time": null
    },
    {
      "partition": 1,
      "current_offset": 1500,
      "log_end_offset": 1500,
      "lag": 0,
      "state": "Active",
      "last_commit_time": null
    }
  ],
  "total_lag": 200
}
```

**字段说明**:
- `current_offset`: 当前已提交的 offset
- `log_end_offset`: 分区最新 offset
- `lag`: 消费滞后量 (log_end_offset - current_offset)
- `state`: 状态 ("Active" 表示有提交，"Empty" 表示无提交)
- `last_commit_time`: 最后提交时间 (毫秒，暂不支持)
- `total_lag`: 总滞后量 (所有分区 lag 之和)

### 重置 Consumer Group Offset

```bash
curl -X POST http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets/reset \
  -H "Content-Type: application/json" \
  -d '{
    "topic": "test-topic",
    "offset": {
      "type": "latest"
    }
  }'
```

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| topic | string | 是 | 要重置 offset 的 topic |
| partition | int | 否 | 分区 ID，不填则重置所有分区 |
| offset | object | 是 | offset 配置，包含 `type` 和 `value` 字段 |

**offset 配置**:
| type | value | 说明 |
|------|-------|------|
| `"earliest"` | 无 | 重置到最早的 offset |
| `"latest"` | 无 | 重置到最新的 offset |
| `"value"` | i64 | 重置到指定的 offset 值 |
| `"timestamp"` | i64 | 重置到指定时间戳对应的 offset |

**示例 - 重置所有分区到最早 offset**:
```bash
curl -X POST http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets/reset \
  -H "Content-Type: application/json" \
  -d '{"topic": "test-topic", "offset": {"type": "earliest"}}'
```

**示例 - 重置指定分区到具体 offset 值**:
```bash
curl -X POST http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets/reset \
  -H "Content-Type: application/json" \
  -d '{"topic": "test-topic", "partition": 0, "offset": {"type": "value", "value": 1000}}'
```

**示例 - 重置指定分区到指定时间戳**:
```bash
curl -X POST http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets/reset \
  -H "Content-Type: application/json" \
  -d '{"topic": "test-topic", "partition": 0, "offset": {"type": "timestamp", "value": 1677628800000}}'
```

### 批量删除 Consumer Group

```bash
curl -X DELETE http://localhost:3000/api/clusters/development/consumer-groups/batch \
  -H "Content-Type: application/json" \
  -d '{
    "group_names": ["group-1", "group-2"],
    "continue_on_error": true
  }'
```

**响应**:
```json
{
  "success": true,
  "deleted": ["group-1", "group-2"],
  "failed": []
}
```

### 查看 Consumer Group 消费速度

```bash
curl "http://localhost:3000/api/clusters/development/consumer-groups/my-group/throughput?topic=test-topic"
```

**响应**:
```json
{
  "group_name": "my-group",
  "topic": "test-topic",
  "consume_throughput": {
    "messages_per_second": 100.5,
    "bytes_per_second": null,
    "window_seconds": 3600
  },
  "total_lag": 5000,
  "estimated_time_to_catch_up": 49.75,
  "partitions": [
    {
      "partition": 0,
      "current_offset": 5000,
      "log_end_offset": 10000,
      "lag": 5000,
      "consume_rate": 80.4
    }
  ]
}
```

**字段说明**:
- `messages_per_second`: 消费速率（条/秒）
- `window_seconds`: 统计时间窗口（秒）
- `total_lag`: 总积压消息数
- `estimated_time_to_catch_up`: 预估追上时间（秒）
- `consume_rate`: 每个分区的消费速率

### 查看 Topic 所有 Consumer Group 积压情况

```bash
curl http://localhost:3000/api/clusters/development/topics/test-topic/consumer-lag
```

**响应**:
```json
{
  "topic": "test-topic",
  "total_lag": 15000,
  "consumer_groups": [
    {
      "group_name": "analytics-consumer",
      "total_lag": 10000,
      "partitions": [
        {
          "partition": 0,
          "current_offset": 5000,
          "log_end_offset": 10000,
          "lag": 5000,
          "state": "Active"
        },
        {
          "partition": 1,
          "current_offset": 5000,
          "log_end_offset": 10000,
          "lag": 5000,
          "state": "Active"
        }
      ]
    },
    {
      "group_name": "etl-consumer",
      "total_lag": 5000,
      "partitions": [
        {
          "partition": 0,
          "current_offset": 7500,
          "log_end_offset": 10000,
          "lag": 2500,
          "state": "Active"
        }
      ]
    }
  ]
}
```

**字段说明**:
- `total_lag`: 所有 Consumer Group 的总积压数
- `consumer_groups`: 按积压数降序排列的 Consumer Group 列表
- `partitions[].state`: "Active" 表示有提交，"Empty" 表示无提交

### 查看 Topic 所有 Consumer Group 历史积压数据（用于折线图）

```bash
# 获取当前时刻的快照数据
curl "http://localhost:3000/api/clusters/development/topics/test-topic/consumer-lag-history"

# 带时间范围参数（前端可以定期调用采集数据点）
curl "http://localhost:3000/api/clusters/development/topics/test-topic/consumer-lag-history?start_time=1708876800000&end_time=1708880400000"
```

**查询参数**:
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| start_time | i64 | 当前时间 | 开始时间戳（毫秒） |
| end_time | i64 | 当前时间 | 结束时间戳（毫秒） |

**响应**:
```json
{
  "topic": "test-topic",
  "start_time": 1708876800000,
  "end_time": 1708880400000,
  "data_points": 1,
  "timestamps": [1708880400000],
  "consumer_groups": [
    {
      "group_name": "analytics-consumer",
      "lag_series": [10000],
      "consumed_series": [10000],
      "produced_series": [20000]
    },
    {
      "group_name": "etl-consumer",
      "lag_series": [5000],
      "consumed_series": [15000],
      "produced_series": [20000]
    }
  ]
}
```

**字段说明**:
- `timestamps`: X 轴时间标签数组（毫秒时间戳）
- `consumer_groups`: 每个 Consumer Group 的历史数据
  - `lag_series`: Y 轴数据，每个时间点的积压数（与 timestamps 对应）
  - `consumed_series`: 每个时间点的已消费 offset 总和
  - `produced_series`: 每个时间点的生产 offset 总和

**前端使用示例**:
```javascript
// 定期调用接口采集数据点，构建实时折线图
async function fetchLagHistory(topic) {
  const response = await fetch(`/api/clusters/dev/topics/${topic}/consumer-lag-history`);
  const data = await response.json();

  // 提取数据用于绘图
  const labels = data.timestamps.map(ts => new Date(ts));
  const datasets = data.consumer_groups.map(group => ({
    label: group.group_name,
    data: group.lag_series,
    borderColor: getRandomColor(),
  }));

  // 使用 Chart.js 或其他库绘制折线图
  renderLineChart(labels, datasets);
}

// 每秒采集一次数据
setInterval(() => fetchLagHistory('test-topic'), 1000);
```

**注意**: 由于 Kafka 不存储历史 offset 数据，此接口返回当前时刻的快照数据。前端需要定期调用此接口（如每秒一次）来采集数据点，构建实时折线图。

---

## 集群监控

### 获取集群信息

```bash
curl http://localhost:3000/api/clusters/development/info
```

**响应**:
```json
{
  "brokers": [
    {
      "id": 1,
      "host": "localhost",
      "port": 9092
    },
    {
      "id": 2,
      "host": "localhost",
      "port": 9093
    }
  ],
  "controller_id": null,
  "cluster_id": null,
  "topic_count": 5,
  "total_partitions": 12
}
```

**字段说明**:
- `brokers`: Broker 列表
- `controller_id`: Controller Broker ID (暂不支持)
- `cluster_id`: 集群 ID (暂不支持)
- `topic_count`: Topic 总数
- `total_partitions`: 所有 Topic 的分区总数

### 获取集群监控指标

```bash
curl http://localhost:3000/api/clusters/development/metrics
```

**响应**:
```json
{
  "broker_count": 3,
  "controller_id": null,
  "topic_count": 5,
  "partition_count": 12,
  "under_replicated_partitions": 0
}
```

**字段说明**:
- `broker_count`: Broker 数量
- `controller_id`: Controller Broker ID (暂不支持)
- `topic_count`: Topic 总数
- `partition_count`: 分区总数
- `under_replicated_partitions`: 未完全复制的分区数 (ISR 数量 < Replica 数量的分区)

### 列出所有 Broker

```bash
curl http://localhost:3000/api/clusters/development/brokers
```

**响应**:
```json
{
  "brokers": [
    {
      "id": 1,
      "host": "localhost",
      "port": 9092
    },
    {
      "id": 2,
      "host": "localhost",
      "port": 9093
    }
  ]
}
```

### 获取 Broker 详情

```bash
curl http://localhost:3000/api/clusters/development/brokers/1
```

**响应**:
```json
{
  "id": 1,
  "host": "localhost",
  "port": 9092,
  "is_controller": false,
  "leader_partitions": 4,
  "replica_partitions": 8
}
```

**字段说明**:
- `id`: Broker ID
- `host`: Broker 主机地址
- `port`: Broker 端口
- `is_controller`: 是否为 Controller
- `leader_partitions`: 作为 Leader 的分区数
- `replica_partitions`: 作为 Replica 的分区数

### 获取 Broker 日志目录

```bash
curl http://localhost:3000/api/clusters/development/brokers/1/logdirs
```

**响应**:
```json
{
  "broker_id": 1,
  "log_dirs": [
    {
      "path": "/var/kafka-logs",
      "size_bytes": 1073741824,
      "topic_count": 5,
      "partition_count": 12
    }
  ]
}
```

### 获取 Broker 详细指标

```bash
curl http://localhost:3000/api/clusters/development/brokers/1/metrics
```

**响应**:
```json
{
  "broker_id": 1,
  "leader_partitions": 4,
  "replica_partitions": 8,
  "is_controller": false,
  "under_replicated_count": 0,
  "offline_partitions": 0
}
```

### 获取集群统计 Dashboard

```bash
curl http://localhost:3000/api/clusters/development/stats
```

**响应**:
```json
{
  "cluster_id": "development",
  "broker_count": 3,
  "controller_id": 1,
  "topic_count": 10,
  "partition_count": 30,
  "under_replicated_partitions": 0,
  "consumer_group_count": 5,
  "total_lag": 1000,
  "broker_stats": [
    {
      "id": 1,
      "host": "localhost",
      "port": 9092,
      "is_controller": true,
      "leader_partitions": 10,
      "replica_partitions": 30
    },
    {
      "id": 2,
      "host": "localhost",
      "port": 9093,
      "is_controller": false,
      "leader_partitions": 10,
      "replica_partitions": 30
    }
  ]
}
```

---

## 健康检查

```bash
curl http://localhost:3000/api/health
```

**响应**:
```json
{
  "status": "healthy"
}
```

---

## Schema Registry 集成

Schema Registry 功能通过 Rust SDK 使用，详见 [README.md](../README.md#schema-registry-集成)。

---

## 认证管理

### 生成新的 API Key

```bash
curl -X POST http://localhost:3000/api/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-app",
    "expires_in_days": 30
  }'
```

**响应**:
```json
{
  "id": 1,
  "key": "km_abc123xyz789...",
  "key_prefix": "km_abc",
  "name": "my-app",
  "created_at": "2026-02-26T10:00:00Z",
  "expires_at": "2026-03-28T10:00:00Z"
}
```

**注意**: `key` 只在创建时返回一次，请妥善保管！

### 列出所有 API Key

```bash
curl http://localhost:3000/api/auth/keys
```

**响应**:
```json
{
  "keys": [
    {
      "id": 1,
      "key_prefix": "km_abc",
      "name": "my-app",
      "created_at": "2026-02-26T10:00:00Z",
      "expires_at": "2026-03-28T10:00:00Z",
      "is_active": true
    }
  ]
}
```

**字段说明**:
- `key_prefix`: API Key 前缀（用于识别）
- `is_active`: 是否启用
- 出于安全考虑，不返回完整的 API Key

### 删除 API Key

```bash
curl -X DELETE http://localhost:3000/api/auth/keys/1
```

**响应**:
```json
{
  "success": true
}
```

---

## 审计日志

### 查询审计日志

```bash
curl "http://localhost:3000/api/audit-logs?limit=50&offset=0"
```

**查询参数**:
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| limit | int | 100 | 每页数量 |
| offset | int | 0 | 偏移量 |
| action | string | 无 | 按操作类型过滤 |
| cluster_id | string | 无 | 按集群 ID 过滤 |
| status | int | 无 | 按状态码过滤 |

**响应**:
```json
{
  "logs": [
    {
      "id": 1,
      "timestamp": "2026-02-26T10:00:00Z",
      "method": "POST",
      "path": "/api/clusters/development/topics",
      "cluster_id": "development",
      "resource": "topic:test-topic",
      "action": "create_topic",
      "api_key": "km_abc***xyz",
      "status": 200,
      "duration_ms": 150,
      "client_ip": "192.168.1.100"
    }
  ],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```

**字段说明**:
- `api_key`: API Key 已脱敏显示
- `resource`: 操作的对象资源
- `action`: 操作类型（create_topic, delete_consumer_group 等）

### 清理旧审计日志

```bash
curl -X DELETE http://localhost:3000/api/audit-logs \
  -H "Content-Type: application/json" \
  -d '{"days": 30}'
```

**响应**:
```json
{
  "deleted": 1000
}
```

---

## 告警规则管理

### 告警规则类型

支持以下告警规则类型：

| 规则类型 | 说明 | 适用场景 |
|---------|------|---------|
| `consumer_lag` | Consumer Group 积压告警 | 监控消费者消费速度 |
| `produce_rate` | Topic 生产速率告警 | 监控生产者发送速率 |
| `consumer_rate` | Consumer Group 消费速率告警 | 监控消费者消费速率 |

### 比较操作符

| 操作符 | 说明 |
|-------|------|
| `gt` | 大于 (>) |
| `lt` | 小于 (<) |
| `gte` | 大于等于 (>=) |
| `lte` | 小于等于 (<=) |
| `eq` | 等于 (=) |

### 告警级别

| 级别 | 说明 |
|------|------|
| `warning` | 警告级别，需要关注 |
| `critical` | 严重级别，需要立即处理 |

### 创建告警规则

```bash
curl -X POST http://localhost:3000/api/alert-rules \
  -H "Content-Type: application/json" \
  -d '{
    "cluster_id": "development",
    "name": "Topic 积压告警",
    "rule_type": "consumer_lag",
    "topic": "order-events",
    "consumer_group": "order-processor",
    "threshold": 10000,
    "comparison": "gt",
    "duration_seconds": 300,
    "severity": "warning",
    "enabled": true,
    "notification_config": "{\"webhook\":\"http://alertmanager:9093/webhook\"}"
  }'
```

**响应**:
```json
{
  "id": 1,
  "cluster_id": "development",
  "name": "Topic 积压告警",
  "rule_type": "consumer_lag",
  "topic": "order-events",
  "consumer_group": "order-processor",
  "threshold": 10000,
  "comparison": "gt",
  "duration_seconds": 300,
  "severity": "warning",
  "enabled": true,
  "notification_config": "{\"webhook\":\"http://alertmanager:9093/webhook\"}",
  "created_at": "2026-02-26T10:00:00Z",
  "updated_at": "2026-02-26T10:00:00Z"
}
```

### 列出告警规则

```bash
# 列出所有规则
curl http://localhost:3000/api/alert-rules

# 按集群过滤
curl "http://localhost:3000/api/alert-rules?cluster_id=development"

# 只看启用的规则
curl "http://localhost:3000/api/alert-rules?enabled=true"
```

**响应**:
```json
[
  {
    "id": 1,
    "cluster_id": "development",
    "name": "Topic 积压告警",
    "rule_type": "consumer_lag",
    "topic": "order-events",
    "consumer_group": "order-processor",
    "threshold": 10000,
    "comparison": "gt",
    "duration_seconds": 300,
    "severity": "warning",
    "enabled": true,
    "notification_config": "{\"webhook\":\"http://alertmanager:9093/webhook\"}",
    "created_at": "2026-02-26T10:00:00Z",
    "updated_at": "2026-02-26T10:00:00Z"
  },
  {
    "id": 2,
    "cluster_id": "development",
    "name": "生产速率过低",
    "rule_type": "produce_rate",
    "topic": "payment-events",
    "consumer_group": null,
    "threshold": 100,
    "comparison": "lt",
    "duration_seconds": 600,
    "severity": "critical",
    "enabled": true,
    "notification_config": null,
    "created_at": "2026-02-26T09:00:00Z",
    "updated_at": "2026-02-26T09:00:00Z"
  }
]
```

### 获取告警规则详情

```bash
curl http://localhost:3000/api/alert-rules/1
```

### 更新告警规则

```bash
# 更新阈值
curl -X PUT http://localhost:3000/api/alert-rules/1 \
  -H "Content-Type: application/json" \
  -d '{"threshold": 20000}'

# 禁用告警
curl -X PUT http://localhost:3000/api/alert-rules/1 \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# 更新通知配置
curl -X PUT http://localhost:3000/api/alert-rules/1 \
  -H "Content-Type: application/json" \
  -d '{"notification_config": "{\"webhook\":\"http://new-alertmanager:9093/webhook\"}"}'
```

### 删除告警规则

```bash
curl -X DELETE http://localhost:3000/api/alert-rules/1
```

### 配置示例

#### Consumer Lag 告警（积压超过 10000 条持续 5 分钟）

```json
{
  "cluster_id": "production",
  "name": "订单处理积压告警",
  "rule_type": "consumer_lag",
  "topic": "order-events",
  "consumer_group": "order-processor",
  "threshold": 10000,
  "comparison": "gt",
  "duration_seconds": 300,
  "severity": "warning",
  "enabled": true
}
```

#### Produce Rate 告警（生产速率低于 100 条/秒持续 10 分钟）

```json
{
  "cluster_id": "production",
  "name": "支付事件生产速率过低",
  "rule_type": "produce_rate",
  "topic": "payment-events",
  "threshold": 100,
  "comparison": "lt",
  "duration_seconds": 600,
  "severity": "critical",
  "enabled": true
}
```

#### Consumer Rate 告警（消费速率低于 50 条/秒）

```json
{
  "cluster_id": "production",
  "name": "日志处理消费过慢",
  "rule_type": "consumer_rate",
  "topic": "application-logs",
  "consumer_group": "log-processor",
  "threshold": 50,
  "comparison": "lt",
  "duration_seconds": 300,
  "severity": "warning",
  "enabled": true
}
```

---

---

## 标签管理

### 概述

标签功能允许为 Topic 和 Consumer Group 添加自定义标签，便于分类、筛选和管理。

**支持的资源类型**:
- `topic`: Topic 资源
- `consumer_group`: Consumer Group 资源

**标签命名规范**:
- 标签键和值均为字符串
- 建议使用小写字母、数字和连字符
- 常用标签示例：`env:production`, `team:data`, `app:orders`

### 获取资源的标签列表

```bash
# 获取 Topic 的标签
curl http://localhost:3000/api/clusters/development/topics/order-events/tags

# 获取 Consumer Group 的标签
curl http://localhost:3000/api/clusters/development/consumer-groups/order-processor/tags
```

**响应**:
```json
[
  {
    "id": 1,
    "cluster_id": "development",
    "resource_type": "topic",
    "resource_name": "order-events",
    "tag_key": "env",
    "tag_value": "production",
    "created_at": "2026-02-26T10:00:00Z",
    "updated_at": "2026-02-26T10:00:00Z"
  },
  {
    "id": 2,
    "cluster_id": "development",
    "resource_type": "topic",
    "resource_name": "order-events",
    "tag_key": "team",
    "tag_value": "orders",
    "created_at": "2026-02-26T10:00:00Z",
    "updated_at": "2026-02-26T10:00:00Z"
  }
]
```

### 为资源添加标签

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/order-events/tags \
  -H "Content-Type: application/json" \
  -d '{
    "key": "env",
    "value": "production"
  }'
```

**响应**:
```json
{
  "id": 1,
  "cluster_id": "development",
  "resource_type": "topic",
  "resource_name": "order-events",
  "tag_key": "env",
  "tag_value": "production",
  "created_at": "2026-02-26T10:00:00Z",
  "updated_at": "2026-02-26T10:00:00Z"
}
```

### 更新资源标签

```bash
curl -X PUT http://localhost:3000/api/clusters/development/topics/order-events/tags/env \
  -H "Content-Type: application/json" \
  -d '{
    "key": "env",
    "value": "staging"
  }'
```

### 删除资源标签

```bash
# 删除单个标签
curl -X DELETE http://localhost:3000/api/clusters/development/topics/order-events/tags/env

# 删除资源的所有标签
curl -X DELETE http://localhost:3000/api/clusters/development/topics/order-events/tags/all
```

**响应**:
```json
{
  "deleted": 2
}
```

### 批量更新资源标签

```bash
curl -X PUT http://localhost:3000/api/clusters/development/topics/order-events/tags/batch \
  -H "Content-Type: application/json" \
  -d '{
    "tags": {
      "env": "production",
      "team": "orders",
      "retention": "7d"
    }
  }'
```

### 列出所有标签键

```bash
# 列出所有标签键
curl "http://localhost:3000/api/clusters/development/tags/keys"

# 列出 Topic 的标签键
curl "http://localhost:3000/api/clusters/development/tags/keys?resource_type=topic"
```

**响应**:
```json
["env", "retention", "team"]
```

### 列出标签键对应的所有值

```bash
curl "http://localhost:3000/api/clusters/development/tags/env/values?resource_type=topic"
```

**响应**:
```json
["production", "staging", "development"]
```

### 按标签过滤资源

```bash
# 获取所有 env=production 的 Topic
curl "http://localhost:3000/api/clusters/development/tags/filter?resource_type=topic&tag_key=env&tag_value=production"

# 获取所有有 env 标签的 Topic（不指定值）
curl "http://localhost:3000/api/clusters/development/tags/filter?resource_type=topic&tag_key=env"
```

**响应**:
```json
["order-events", "payment-events", "user-events"]
```

### 使用场景示例

#### 按环境分类
```bash
# 为生产环境 Topic 添加标签
curl -X POST http://localhost:3000/api/clusters/production/topics/order-events/tags \
  -H "Content-Type: application/json" \
  -d '{"key": "env", "value": "production"}'
```

#### 按团队分类
```bash
# 为订单团队的 Topic 添加标签
curl -X POST http://localhost:3000/api/clusters/development/topics/order-events/tags \
  -H "Content-Type: application/json" \
  -d '{"key": "team", "value": "orders"}'
```

#### 按业务分类
```bash
# 批量添加多个标签
curl -X PUT http://localhost:3000/api/clusters/development/topics/order-events/tags/batch \
  -H "Content-Type: application/json" \
  -d '{
    "tags": {
      "env": "production",
      "team": "orders",
      "business": "ecommerce",
      "criticality": "high"
    }
  }'
```

---

---

## Topic 配置模板

### 概述

Topic 配置模板功能允许预定义 Topic 配置模板，使用模板一键创建标准化 Topic。

### 预定义模板

系统内置 5 个预定义模板：

| 模板名称 | 分区数 | 副本数 | 说明 |
|---------|-------|-------|------|
| `default` | 3 | 1 | 默认模板，保留 7 天 |
| `high-throughput` | 12 | 3 | 高吞吐模板，保留 3 天，LZ4 压缩 |
| `event-sourcing` | 6 | 3 | 事件溯源模板，永久保留，compaction |
| `logging` | 6 | 2 | 日志模板，保留 1 天，GZIP 压缩 |
| `dead-letter` | 3 | 3 | 死信队列模板，保留 30 天 |

### 列出预定义模板

```bash
curl http://localhost:3000/api/topic-templates/presets
```

**响应**:
```json
[
  {
    "name": "default",
    "description": "默认模板：3 分区，1 副本，保留 7 天",
    "num_partitions": 3,
    "replication_factor": 1,
    "config": {
      "retention.ms": "604800000",
      "segment.bytes": "1073741824"
    }
  },
  {
    "name": "high-throughput",
    "description": "高吞吐模板：12 分区，3 副本，保留 3 天",
    "num_partitions": 12,
    "replication_factor": 3,
    "config": {
      "retention.ms": "259200000",
      "segment.bytes": "536870912",
      "compression.type": "lz4"
    }
  }
]
```

### 创建自定义模板

```bash
curl -X POST http://localhost:3000/api/topic-templates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-custom-template",
    "description": "我的自定义模板",
    "num_partitions": 6,
    "replication_factor": 2,
    "config": {
      "retention.ms": "172800000",
      "cleanup.policy": "delete"
    }
  }'
```

### 列出所有自定义模板

```bash
curl http://localhost:3000/api/topic-templates
```

### 获取单个模板

```bash
curl http://localhost:3000/api/topic-templates/1
```

### 更新模板

```bash
curl -X PUT http://localhost:3000/api/topic-templates/1 \
  -H "Content-Type: application/json" \
  -d '{
    "num_partitions": 9,
    "config": {
      "retention.ms": "259200000"
    }
  }'
```

### 删除模板

```bash
curl -X DELETE http://localhost:3000/api/topic-templates/1
```

### 使用模板创建 Topic

```bash
# 使用模板名称
curl -X POST "http://localhost:3000/api/topics/from-template?cluster_id=development" \
  -H "Content-Type: application/json" \
  -d '{
    "topic_name": "new-order-topic",
    "template_name": "high-throughput"
  }'

# 使用模板 ID
curl -X POST "http://localhost:3000/api/topics/from-template?cluster_id=development" \
  -H "Content-Type: application/json" \
  -d '{
    "topic_name": "new-order-topic",
    "template_id": 1
  }'

# 使用模板并覆盖配置
curl -X POST "http://localhost:3000/api/topics/from-template?cluster_id=development" \
  -H "Content-Type: application/json" \
  -d '{
    "topic_name": "new-order-topic",
    "template_name": "default",
    "override_config": {
      "retention.ms": "1209600000"
    }
  }'
```

**响应**:
```json
{
  "success": true,
  "topic": "new-order-topic",
  "template": "high-throughput",
  "num_partitions": 12,
  "replication_factor": 3
}
```

---
