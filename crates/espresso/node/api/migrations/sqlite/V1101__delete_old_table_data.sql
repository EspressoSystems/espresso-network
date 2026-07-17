-- Delete data from old consensus tables
-- The data has been migrated to the new tables (anchor_leaf2, da_proposal2,
-- vid_share2, quorum_proposals2, quorum_certificate2)
DELETE FROM anchor_leaf;
DELETE FROM da_proposal;
DELETE FROM vid_share;
DELETE FROM quorum_proposals;
DELETE FROM quorum_certificate;
