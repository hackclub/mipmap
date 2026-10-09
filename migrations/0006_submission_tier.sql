ALTER TABLE submission ADD COLUMN tier TEXT;

UPDATE submission s SET tier = p.tier FROM project p WHERE p.id = s.project_id;
