-- 创建 Kafka 集群表
CREATE TABLE IF NOT EXISTS kafka_clusters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    brokers TEXT NOT NULL,
    request_timeout_ms INTEGER NOT NULL DEFAULT 5000,
    operation_timeout_ms INTEGER NOT NULL DEFAULT 5000,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_kafka_clusters_name ON kafka_clusters(name);
