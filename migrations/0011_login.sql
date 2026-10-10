CREATE TABLE login (
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    logged_in_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX login_logged_in_at ON login (logged_in_at);

INSERT INTO login (user_id, logged_in_at)
SELECT user_id, created_at FROM session
UNION
SELECT id, last_login_at FROM "user" WHERE last_login_at IS NOT NULL;
