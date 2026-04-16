-- Add information needed for consensus storage to act as a provider for archive recovery.

-- Add payload hash to DA proposal, since the query service requests missing payloads by hash.
ALTER TABLE da_proposal
    ADD COLUMN payload_hash VARCHAR;
CREATE INDEX da_proposal_payload_hash_idx ON da_proposal (payload_hash);

-- Add payload hash to VID share, since the query service requests missing VID common by payload
-- hash.
ALTER TABLE vid_share
    ADD COLUMN payload_hash VARCHAR;
CREATE INDEX vid_share_payload_hash_idx ON vid_share (payload_hash);

-- Add QC storage, since the query service requires missing leaves to be fetched alongside a QC with
-- that leaf hash.
CREATE TABLE quorum_certificate (
    view BIGINT PRIMARY KEY,
    leaf_hash VARCHAR NOT NULL,
    data BLOB NOT NULL
);
CREATE INDEX quorum_certificate_leaf_hash_idx ON quorum_certificate (leaf_hash);
