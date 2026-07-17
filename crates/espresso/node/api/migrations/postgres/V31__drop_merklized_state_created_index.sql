-- Indexes on created block height are not strictly necessary for Merklized state queries. In most
-- cases, we want the queries to use the multi-column index on (path, created), which allows us to
-- seek directly to the desired path down the tree and then take the latest version of that path in
-- a single B-tree traversal.
--
-- Occasionally, it is marginally faster to use the created index, such as when a Merkle node was
-- modified very recently, and you don't have to scan back very far in the created index before
-- finding a version of that node. However, it is sometimes *much* slower to use `created` over the
-- multi-column index, such as when a node hasn't had a new version in a very long time. For reasons
-- that are not well understood, having these indexes causes the query planner to sometimes use them
-- in the extremely slow cases, but dropping them means we always use the multi-column index.
DROP INDEX fee_merkle_tree_created;
DROP INDEX block_merkle_tree_created;
