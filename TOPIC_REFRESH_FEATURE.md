# Topic 刷新功能实现

## 新增功能

### 1. 刷新集群 Topic 列表 API

**端点**: `POST /api/clusters/:cluster_id/topics/refresh`

**功能**: 从 Kafka 集群实时同步 Topic 列表到数据库

**响应示例**:
```json
{
  "success": true,
  "added": ["new-topic-1", "new-topic-2"],
  "removed": ["deleted-topic"],
  "total": 15
}
```

**字段说明**:
- `success`: 同步是否成功
- `added`: 新增的 Topic 列表（之前在 Kafka 中存在但数据库中没有）
- `removed`: 删除的 Topic 列表（之前在数据库中存在但 Kafka 中已删除）
- `total`: 当前数据库中的 Topic 总数

### 2. 刷新 Cluster 时自动同步 Topic

当执行以下操作时，会自动同步该集群的 Topic 列表：
- 创建新集群 (`POST /api/clusters`)
- 更新集群配置 (`PUT /api/clusters/:id`)
- 删除集群 (`DELETE /api/clusters/:id`)

**实现逻辑**:
```rust
// 在 reload_clients 函数中
for cluster in &clusters {
    if let Some(admin) = new_clients.get_admin(&cluster.name) {
        if let Ok(topics) = admin.list_topics() {
            let _ = TopicStore::sync_topics(state.db.inner(), &cluster.name, &topics).await;
            tracing::info!("Synced {} topics for cluster '{}'", topics.len(), cluster.name);
        }
    }
}
```

## 数据库变更

### 新增表：topic_metadata

```sql
CREATE TABLE IF NOT EXISTS topic_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    partition_count INTEGER NOT NULL DEFAULT 1,
    replication_factor INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
)
```

**索引**:
- `idx_topic_metadata_cluster`: 按 cluster_id 查询
- `idx_topic_metadata_name`: 按 topic_name 查询

## 新增文件

### `src/db/topic.rs`

Topic 元数据存储模块，提供以下功能：

```rust
pub struct TopicStore;

impl TopicStore {
    // 保存或更新 Topic 元数据
    pub async fn upsert(...) -> Result<i64>

    // 获取集群的所有 Topic 元数据
    pub async fn list_by_cluster(...) -> Result<Vec<TopicMetadata>>

    // 获取指定 Topic 元数据
    pub async fn get_by_name(...) -> Result<Option<TopicMetadata>>

    // 删除 Topic 元数据
    pub async fn delete(...) -> Result<bool>

    // 同步集群 Topic 列表
    pub async fn sync_topics(...) -> Result<SyncResult>
}
```

## 修改文件

### `src/routes/topic.rs`

新增刷新端点和处理函数：
- 添加路由：`.route("/refresh", post(refresh_topics))`
- 新增结构体：`RefreshTopicsResponse`
- 新增处理函数：`refresh_topics()`

### `src/routes/cluster.rs`

修改 `reload_clients()` 函数，在重新加载 Kafka 客户端时自动同步 Topic 列表。

### `src/db/mod.rs`

- 添加模块声明：`pub mod topic;`
- 添加表创建语句

## 使用示例

### 1. 手动刷新 Topic 列表

```bash
# 刷新指定集群的 Topic 列表
curl -X POST http://localhost:9732/api/clusters/cluster-1/topics/refresh
```

### 2. 添加集群时自动同步

```bash
# 创建集群时会自动同步 Topic
curl -X POST http://localhost:9732/api/clusters \
  -H "Content-Type: application/json" \
  -d '{
    "name": "cluster-1",
    "brokers": "localhost:9092",
    "request_timeout_ms": 5000,
    "operation_timeout_ms": 5000
  }'
```

### 3. 更新集群时自动同步

```bash
# 更新集群配置时会自动同步 Topic
curl -X PUT http://localhost:9732/api/clusters/1 \
  -H "Content-Type: application/json" \
  -d '{
    "brokers": "localhost:9092,localhost:9093"
  }'
```

## 同步逻辑

```
┌─────────────────────────────────────────────────────────────┐
│                    Topic 同步流程                            │
└─────────────────────────────────────────────────────────────┘

1. 从 Kafka 集群获取当前 Topic 列表
   ↓
2. 从数据库获取已存在的 Topic 记录
   ↓
3. 比较差异：
   - 新增：Kafka 存在但数据库不存在
   - 删除：数据库存在但 Kafka 不存在
   ↓
4. 删除已不存在的 Topic 记录
   ↓
5. 返回同步结果（added, removed, total）
```

## 日志输出

成功同步时会输出日志：
```
2026-02-26T16:42:02.708697Z  INFO  Synced 15 topics for cluster 'cluster-1'
```

## 注意事项

1. **性能考虑**: 同步操作会遍历 Kafka 集群的所有 Topic，如果 Topic 数量很多，可能需要几秒时间。

2. **数据一致性**: 同步操作使用数据库事务，确保数据一致性。

3. **自动清理**: 当 Topic 在 Kafka 中被删除时，下次同步会自动清理数据库中的记录。

4. **扩展性**: 当前实现仅同步 Topic 基本信息（名称、分区数、副本数），如需同步更多元数据（如配置、分区详情），可以扩展 `upsert` 函数。

## TODO

- [ ] 支持只刷新特定 Topic
- [ ] 支持定时自动刷新（后台任务）
- [ ] 支持同步 Topic 配置信息
- [ ] 支持同步分区详情
