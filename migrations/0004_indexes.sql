CREATE INDEX session_user_id ON session (user_id);
CREATE INDEX project_user_id ON project (user_id);
CREATE INDEX snapshot_user_id ON snapshot (user_id);
CREATE UNIQUE INDEX order_user_node ON "order" (user_id, node_id) WHERE deleted_at IS NULL;
