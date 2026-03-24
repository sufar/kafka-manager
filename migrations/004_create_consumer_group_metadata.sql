-- Consumer Group Metadata 表
-- 用于缓存 Consumer Group 元数据（group_name, cluster_id, topics）
CREATE TABLE IF NOT EXISTS consumer_group_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    group_name TEXT NOT NULL,
    topics TEXT NOT NULL,  -- JSON 数组，存储该消费组订阅的所有 topic
    fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(cluster_id, group_name)
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_consumer_group_metadata_cluster_id ON consumer_group_metadata(cluster_id);
CREATE INDEX IF NOT EXISTS idx_consumer_group_metadata_group_name ON consumer_group_metadata(group_name);
CREATE INDEX IF NOT EXISTS idx_consumer_group_metadata_cluster_group ON consumer_group_metadata(cluster_id, group_name);

-- Consumer Group Offset 缓存表（用于历史记录）
CREATE TABLE IF NOT EXISTS consumer_group_offsets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    group_name TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    partition_id INTEGER NOT NULL,
    offset_value INTEGER NOT NULL,
    lag INTEGER NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(cluster_id, group_name, topic_name, partition_id)
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_consumer_group_offsets_cluster_group ON consumer_group_offsets(cluster_id, group_name);
CREATE INDEX IF NOT EXISTS idx_consumer_group_offsets_topic ON consumer_group_offsets(cluster_id, topic_name);
