-- 创建发送消息历史表
CREATE TABLE IF NOT EXISTS sent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    partition INTEGER NOT NULL,
    message_key TEXT,
    message_value TEXT NOT NULL,
    headers TEXT,
    offset INTEGER,
    sent_at TEXT NOT NULL
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sent_messages_sent_at ON sent_messages(sent_at DESC);
CREATE INDEX IF NOT EXISTS idx_sent_messages_cluster_topic ON sent_messages(cluster_id, topic_name, sent_at DESC);
