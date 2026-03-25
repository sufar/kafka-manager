-- Add last_commit_time column to consumer_group_offsets table
ALTER TABLE consumer_group_offsets ADD COLUMN last_commit_time INTEGER;
