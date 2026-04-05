-- Consumer Lag 历史数据索引
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_topic ON consumer_lag_history(topic);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_timestamp ON consumer_lag_history(timestamp);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_topic_timestamp ON consumer_lag_history(topic, timestamp);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_group ON consumer_lag_history(group_name);

-- 告警规则索引
CREATE INDEX IF NOT EXISTS idx_alert_rules_cluster_id ON alert_rules(cluster_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_alert_rules_topic ON alert_rules(topic);

-- Topic 标签索引
CREATE INDEX IF NOT EXISTS idx_tags_cluster_id ON tags(cluster_id);
CREATE INDEX IF NOT EXISTS idx_tags_resource ON tags(resource_type, resource_name);
CREATE INDEX IF NOT EXISTS idx_tags_key ON tags(tag_key);
CREATE INDEX IF NOT EXISTS idx_tags_key_value ON tags(tag_key, tag_value);
