CREATE VIEW project_hackatime AS
SELECT
    p.id,
    GREATEST(p.updated_at, u.hackatime_linked_at) AS changed_at,
    u.hackatime_user_id
FROM project p
JOIN "user" u ON u.id = p.user_id;
