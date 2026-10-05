CREATE TABLE feedback (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    score SMALLINT NOT NULL CHECK (score BETWEEN 1 AND 10),
    source TEXT,
    positives TEXT,
    improvements TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TRIGGER feedback_touch BEFORE UPDATE ON feedback
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
