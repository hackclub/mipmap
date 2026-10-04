CREATE FUNCTION touch_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION random_referral_code() RETURNS text AS $$
    SELECT string_agg(substr('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', floor(random() * 62)::int + 1, 1), '')
    FROM generate_series(1, 10);
$$ LANGUAGE sql VOLATILE;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    hca_id TEXT NOT NULL UNIQUE,
    first_name TEXT,
    last_name TEXT,
    email TEXT,
    verification_status TEXT,
    ysws_eligible BOOLEAN NOT NULL DEFAULT false,
    birthday DATE,
    phone_number TEXT,
    slack_id TEXT,
    address_first_name TEXT,
    address_last_name TEXT,
    address_line_1 TEXT,
    address_line_2 TEXT,
    address_city TEXT,
    address_state TEXT,
    address_postal_code TEXT,
    address_country TEXT,
    address_phone_number TEXT,
    referral_code TEXT NOT NULL UNIQUE DEFAULT random_referral_code(),
    referred_by INTEGER REFERENCES users (id),
    fragments INTEGER NOT NULL DEFAULT 0,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    is_banned BOOLEAN NOT NULL DEFAULT false,
    hackatime_user_id BIGINT,
    hackatime_trust_level TEXT,
    hackatime_baseline JSONB,
    timezone TEXT,
    signup_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ,
    last_activity_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE auth_tokens (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    provider TEXT NOT NULL CHECK (provider IN ('hca', 'hackatime')),
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, provider)
);

CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE unlocks (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    node_id INTEGER NOT NULL,
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    UNIQUE (user_id, node_id)
);

CREATE TABLE snapshots (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    display_name TEXT NOT NULL,
    caption TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL,
    hackatime_totals JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    repo_url TEXT,
    demo_url TEXT,
    screenshot_url TEXT,
    tier TEXT,
    hackatime_projects TEXT[] NOT NULL DEFAULT '{}',
    total_seconds BIGINT NOT NULL DEFAULT 0,
    last_logged_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'draft',
    shipped_at TIMESTAMPTZ,
    review_feedback TEXT,
    fragments_awarded INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    node_id INTEGER NOT NULL,
    item_name TEXT NOT NULL,
    cost INTEGER NOT NULL,
    ordered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    fulfilled BOOLEAN NOT NULL DEFAULT false,
    fulfilled_at TIMESTAMPTZ,
    tracking_number TEXT,
    address_first_name TEXT,
    address_last_name TEXT,
    address_line_1 TEXT,
    address_line_2 TEXT,
    address_city TEXT,
    address_state TEXT,
    address_postal_code TEXT,
    address_country TEXT,
    address_phone_number TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE airtable_sync (
    table_name TEXT PRIMARY KEY,
    synced_until TIMESTAMPTZ NOT NULL
);

CREATE TRIGGER users_touch BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER auth_tokens_touch BEFORE UPDATE ON auth_tokens FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER unlocks_touch BEFORE UPDATE ON unlocks FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER snapshots_touch BEFORE UPDATE ON snapshots FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER projects_touch BEFORE UPDATE ON projects FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER orders_touch BEFORE UPDATE ON orders FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
