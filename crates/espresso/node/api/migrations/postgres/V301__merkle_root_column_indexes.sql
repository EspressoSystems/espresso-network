-- Migration V16 created these columns and indexed them. Then, migration V36 altered the expression
-- use to populate the generated columns. However, it did so by dropping and re-adding the columns
-- with a different expression, which also caused the indexes on the columns to be dropped. They
-- were erroneously not added back. This migration corrects that error by recreating the indexes.
CREATE INDEX header_block_merkle_tree_root_idx ON header (block_merkle_tree_root);
CREATE INDEX header_fee_merkle_tree_root_idx ON header (fee_merkle_tree_root);
