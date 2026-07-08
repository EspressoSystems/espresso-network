ALTER TABLE leaf ADD COLUMN last_used BIGINT NOT NULL DEFAULT 0;
CREATE INDEX leaf_last_used ON leaf (last_used);
