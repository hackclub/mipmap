CREATE INDEX snapshot_feed ON snapshot (created_at DESC) WHERE deleted_at IS NULL AND shadowbanned_at IS NULL;
