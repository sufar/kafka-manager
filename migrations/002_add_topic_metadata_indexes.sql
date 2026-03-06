-- Topic Metadata 索引优化
-- 为 topic_name 创建索引，支持搜索
CREATE INDEX IF NOT EXISTS idx_topic_metadata_topic_name ON topic_metadata(topic_name);

-- 为 cluster_id 创建索引，支持按集群查询
CREATE INDEX IF NOT EXISTS idx_topic_metadata_cluster_id ON topic_metadata(cluster_id);

-- 为 cluster_id + topic_name 创建复合索引，支持ORDER BY优化
CREATE INDEX IF NOT EXISTS idx_topic_metadata_cluster_topic ON topic_metadata(cluster_id, topic_name);
