-- Create consumer_group_lag_history table for tracking lag over time
CREATE TABLE IF NOT EXISTS consumer_group_lag_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cluster_id TEXT NOT NULL,
    group_name TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    total_lag INTEGER NOT NULL,
    topic TEXT NOT NULL,
    partition INTEGER NOT NULL,
    partition_lag INTEGER NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_lag_history_cluster_group ON consumer_group_lag_history(cluster_id, group_name);
CREATE INDEX IF NOT EXISTS idx_lag_history_timestamp ON consumer_group_lag_history(timestamp);
CREATE INDEX IF NOT EXISTS idx_lag_history_cluster_group_timestamp ON consumer_group_lag_history(cluster_id, group_name, timestamp);
