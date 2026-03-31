-- 创建 Topic 浏览历史表
CREATE TABLE IF NOT EXISTS topic_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    viewed_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_topic_history_viewed_at ON topic_history(viewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_topic_history_cluster ON topic_history(cluster_id, viewed_at DESC);
