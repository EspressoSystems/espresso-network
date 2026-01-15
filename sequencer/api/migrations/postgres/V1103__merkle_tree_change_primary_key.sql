-- Change primary key to just the path

-- reward_merkle_tree

ALTER TABLE 
  reward_merkle_tree
DROP 
  CONSTRAINT IF EXISTS reward_merkle_tree_pk;

ALTER TABLE 
  reward_merkle_tree
ADD 
  CONSTRAINT reward_merkle_tree_pkey PRIMARY KEY (path);

-- reward_merkle_tree_v2

ALTER TABLE 
  reward_merkle_tree_v2
DROP 
  CONSTRAINT IF EXISTS reward_merkle_tree_v2_pkey;

ALTER TABLE 
  reward_merkle_tree_v2
ADD 
  CONSTRAINT reward_merkle_tree_v2_pkey PRIMARY KEY (path);

-- fee_merkle_tree 

ALTER TABLE 
  fee_merkle_tree
DROP 
  CONSTRAINT IF EXISTS fee_merkle_tree_pkey;

ALTER TABLE 
  fee_merkle_tree
ADD 
  CONSTRAINT fee_merkle_tree_pkey PRIMARY KEY (path);

-- block_merkle_tree

ALTER TABLE 
  block_merkle_tree
DROP 
  CONSTRAINT IF EXISTS block_merkle_tree_pk;

ALTER TABLE 
  block_merkle_tree
ADD 
  CONSTRAINT block_merkle_tree_pkey PRIMARY KEY (path);
