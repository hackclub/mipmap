ALTER TABLE "user"
    ADD COLUMN saw_old_onboarding BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN onboarding_step TEXT,
    ADD COLUMN onboarding_started_at TIMESTAMPTZ,
    ADD COLUMN onboarding_granted_at TIMESTAMPTZ,
    ADD COLUMN onboarded_at TIMESTAMPTZ,
    ADD COLUMN sticker_skipped_at TIMESTAMPTZ;

UPDATE "user" SET saw_old_onboarding = guides_seen;

ALTER TABLE "user" DROP COLUMN guides_seen;
