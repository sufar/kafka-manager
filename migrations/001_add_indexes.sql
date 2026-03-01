-- 审计日志索引优化
CREATE INDEX IF NOT EXISTS idx_audit_logs_cluster_id ON audit_logs(cluster_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_cluster_timestamp ON audit_logs(cluster_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_status ON audit_logs(status);

-- Consumer Lag 历史数据索引
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_topic ON consumer_lag_history(topic);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_timestamp ON consumer_lag_history(timestamp);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_topic_timestamp ON consumer_lag_history(topic, timestamp);
CREATE INDEX IF NOT EXISTS idx_consumer_lag_history_group ON consumer_lag_history(group_name);

-- 告警规则索引
CREATE INDEX IF NOT EXISTS idx_alert_rules_cluster_id ON alert_rules(cluster_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_alert_rules_topic ON alert_rules(topic);

-- API Key 索引
CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix);
CREATE INDEX IF NOT EXISTS idx_api_keys_is_active ON api_keys(is_active);

-- Topic 标签索引
CREATE INDEX IF NOT EXISTS idx_tags_cluster_id ON tags(cluster_id);
CREATE INDEX IF NOT EXISTS idx_tags_resource ON tags(resource_type, resource_name);
CREATE INDEX IF NOT EXISTS idx_tags_key ON tags(tag_key);
CREATE INDEX IF NOT EXISTS idx_tags_key_value ON tags(tag_key, tag_value);
