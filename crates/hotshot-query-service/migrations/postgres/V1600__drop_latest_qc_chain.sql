-- The new protocol stores no legacy QC chain, and nothing has read it since the light client
-- stopped falling back to it.
--
-- Numbered V1600 rather than the next free query-service slot so it sorts after the node's
-- V15xx migrations, which share one refinery sequence with these. Refinery aborts startup on a
-- new migration numbered below the newest applied one, so node migrations must be V1601 or
-- higher from now on.
DROP TABLE IF EXISTS latest_qc_chain;
