CREATE TABLE sticker_wave (
    id SERIAL PRIMARY KEY,
    admin_id INTEGER NOT NULL REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE "order" ADD COLUMN sticker_wave_id INTEGER REFERENCES sticker_wave (id);
CREATE INDEX order_sticker_wave ON "order" (sticker_wave_id);
