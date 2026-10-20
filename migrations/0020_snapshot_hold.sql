ALTER TABLE snapshot ADD COLUMN held_image BYTEA, ADD COLUMN hold_reasons TEXT[] NOT NULL DEFAULT '{}';

CREATE INDEX snapshot_held ON snapshot (id) WHERE held_image IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE snapshot_rejection (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    display_name TEXT NOT NULL,
    caption TEXT NOT NULL,
    kind TEXT NOT NULL,
    categories TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
