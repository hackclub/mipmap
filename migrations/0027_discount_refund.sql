UPDATE "user" SET mips = "user".mips + refund.mips
FROM (
    SELECT user_id, sum(price - (price * 85 + 50) / 100) AS mips
    FROM unlock
    WHERE deleted_at IS NULL AND price IS NOT NULL
        AND (node_id BETWEEN 5 AND 39 OR node_id BETWEEN 55 AND 60)
    GROUP BY user_id
) refund
WHERE "user".id = refund.user_id;

UPDATE unlock SET price = (price * 85 + 50) / 100
WHERE deleted_at IS NULL
    AND (node_id BETWEEN 5 AND 39 OR node_id BETWEEN 55 AND 60);
