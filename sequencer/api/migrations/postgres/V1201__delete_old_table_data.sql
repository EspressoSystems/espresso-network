-- Delete data from old consensus tables
-- The data has been migrated to the new tables (anchor_leaf2, da_proposal2,
-- vid_share2, quorum_proposals2, quorum_certificate2)
TRUNCATE anchor_leaf;
TRUNCATE da_proposal;
TRUNCATE vid_share;
TRUNCATE quorum_proposals;
TRUNCATE quorum_certificate;
