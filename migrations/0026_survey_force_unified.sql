ALTER TABLE "user" ADD COLUMN survey_forced BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE feedback ADD COLUMN unified_record_id TEXT;
