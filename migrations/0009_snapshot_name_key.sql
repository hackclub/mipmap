ALTER TABLE snapshot ADD COLUMN name_key TEXT;
CREATE INDEX snapshot_name_key ON snapshot (name_key) WHERE deleted_at IS NULL;
