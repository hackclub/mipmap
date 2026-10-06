ALTER TABLE "user" ADD COLUMN hackatime_linked_at TIMESTAMPTZ;
ALTER TABLE submission ADD COLUMN approved_at TIMESTAMPTZ;

UPDATE "user" u SET hackatime_linked_at = t.updated_at FROM auth_token t
WHERE t.user_id = u.id AND t.provider = 'hackatime' AND u.hackatime_user_id IS NOT NULL;
UPDATE submission SET approved_at = updated_at WHERE review_status = '2–Submitted' AND NOT rejected;

CREATE FUNCTION stamp_fulfilled_at() RETURNS trigger AS $$
BEGIN
    NEW.fulfilled_at = CASE WHEN NEW.fulfilled THEN COALESCE(NEW.fulfilled_at, now()) END;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER order_fulfilled_at BEFORE INSERT OR UPDATE ON "order"
    FOR EACH ROW EXECUTE FUNCTION stamp_fulfilled_at();

UPDATE "order" SET fulfilled_at = updated_at WHERE fulfilled AND fulfilled_at IS NULL;

CREATE VIEW user_loops AS
SELECT
    u.id,
    GREATEST(u.updated_at, s.updated_at, o.updated_at) AS changed_at,
    u.signup_at AS "Loops - mipmapSignedUpAt",
    u.last_activity_at AS "Loops - mipmapLastActiveAt",
    u.hackatime_linked_at AS "Loops - mipmapHackatimeLinkedAt",
    s.last_at AS "Loops - mipmapLastShippedAt",
    s.last_approved_at AS "Loops - mipmapLastApprovedAt",
    u.mips AS "Loops - mipmapMipsBalance",
    o.last_at AS "Loops - mipmapLastOrderAt"
FROM "user" u
CROSS JOIN LATERAL (
    SELECT max(sub.updated_at) AS updated_at,
        max(sub.created_at) FILTER (WHERE sub.deleted_at IS NULL) AS last_at,
        max(sub.approved_at) FILTER (WHERE sub.deleted_at IS NULL) AS last_approved_at
    FROM submission sub JOIN project p ON p.id = sub.project_id WHERE p.user_id = u.id
) s
CROSS JOIN LATERAL (
    SELECT max(updated_at) AS updated_at,
        max(ordered_at) FILTER (WHERE deleted_at IS NULL) AS last_at
    FROM "order" WHERE user_id = u.id
) o
WHERE u.deleted_at IS NULL;
