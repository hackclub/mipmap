ALTER TABLE submission
    ADD COLUMN bonus_percent DOUBLE PRECISION NOT NULL DEFAULT 0,
    ADD COLUMN bonus_mips INTEGER NOT NULL DEFAULT 0;

ALTER TABLE snapshot ADD COLUMN shadowbanned_at TIMESTAMPTZ;

CREATE TABLE admin_action (
    id SERIAL PRIMARY KEY,
    admin_id INTEGER NOT NULL REFERENCES "user" (id),
    action TEXT NOT NULL,
    user_id INTEGER REFERENCES "user" (id),
    summary TEXT NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
