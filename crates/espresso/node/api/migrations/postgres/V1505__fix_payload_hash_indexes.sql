DROP INDEX IF EXISTS da_proposal2_payload_hash_idx;
DROP INDEX IF EXISTS vid_share2_payload_hash_idx;

CREATE INDEX da_proposal2_payload_hash_idx ON da_proposal2 (payload_hash);
CREATE INDEX vid_share2_payload_hash_idx ON vid_share2 (payload_hash);
