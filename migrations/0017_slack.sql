CREATE TABLE slack_preference (
    user_id INTEGER PRIMARY KEY REFERENCES "user" (id),
    enabled BOOLEAN NOT NULL DEFAULT true,
    announcements BOOLEAN NOT NULL DEFAULT true,
    challenges BOOLEAN NOT NULL DEFAULT true,
    fulfillment_rounds BOOLEAN NOT NULL DEFAULT true,
    snapshot_reminders BOOLEAN NOT NULL DEFAULT false,
    ship_updates BOOLEAN NOT NULL DEFAULT false,
    inactivity_reminders BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO slack_preference (user_id) SELECT id FROM "user";

CREATE FUNCTION create_slack_preference() RETURNS trigger AS $$
BEGIN
    INSERT INTO slack_preference (user_id) VALUES (NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_slack_preference AFTER INSERT ON "user"
    FOR EACH ROW EXECUTE FUNCTION create_slack_preference();

CREATE TABLE slack_announcement (
    id SERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    dedupe_key TEXT UNIQUE,
    admin_id INTEGER REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at TIMESTAMPTZ,
    dismissed_at TIMESTAMPTZ
);

CREATE TABLE slack_message (
    id SERIAL PRIMARY KEY,
    slack_id TEXT NOT NULL,
    user_id INTEGER REFERENCES "user" (id),
    incoming BOOLEAN NOT NULL DEFAULT false,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    dedupe_key TEXT UNIQUE,
    announcement_id INTEGER REFERENCES slack_announcement (id),
    admin_id INTEGER REFERENCES "user" (id),
    ts TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    sent_at TIMESTAMPTZ,
    skipped_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    error TEXT,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slack_id, ts)
);

CREATE INDEX slack_message_queue ON slack_message ((announcement_id IS NOT NULL), id)
    WHERE NOT incoming AND sent_at IS NULL AND skipped_at IS NULL AND failed_at IS NULL;
CREATE INDEX slack_message_thread ON slack_message (slack_id, created_at);
CREATE INDEX slack_message_announcement ON slack_message (announcement_id);

ALTER TABLE submission ADD COLUMN status_notified TEXT;
ALTER TABLE submission DISABLE TRIGGER submission_touch;
UPDATE submission SET status_notified = CASE
    WHEN rejected THEN 'rejected'
    WHEN approved_at IS NOT NULL AND hours IS NOT NULL THEN 'approved'
END;
ALTER TABLE submission ENABLE TRIGGER submission_touch;
