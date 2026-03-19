-- 添加集群分组表
CREATE TABLE IF NOT EXISTS cluster_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 为 kafka_clusters 表添加 group_id 字段
ALTER TABLE kafka_clusters ADD COLUMN group_id INTEGER REFERENCES cluster_groups(id);

-- 为 cluster_groups 创建索引
CREATE INDEX IF NOT EXISTS idx_cluster_groups_name ON cluster_groups(name);
CREATE INDEX IF NOT EXISTS idx_cluster_groups_sort_order ON cluster_groups(sort_order);

-- 为 kafka_clusters.group_id 创建索引
CREATE INDEX IF NOT EXISTS idx_kafka_clusters_group_id ON kafka_clusters(group_id);
