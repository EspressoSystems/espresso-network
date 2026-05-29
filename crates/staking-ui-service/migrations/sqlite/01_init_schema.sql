 
-- L1 Block tracking
-- only one row, updated for each finalized block
CREATE TABLE l1_block (
    hash TEXT PRIMARY KEY,
    number BIGINT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    `timestamp` BIGINT NOT NULL,
    exit_escrow_period BIGINT NOT NULL
);

-- Full node Set
-- Stores the finalized node set according to L1 state
CREATE TABLE node (
    address TEXT PRIMARY KEY,
    staking_key TEXT NOT NULL UNIQUE,
    state_key TEXT NOT NULL UNIQUE,
    stake TEXT NOT NULL,
    commission REAL NOT NULL
);

-- Wallet State
-- Stores the latest finalized state for each wallet
CREATE TABLE wallet (
    address TEXT PRIMARY KEY,
    claimed_rewards TEXT NOT NULL
);

-- Active Delegations
CREATE TABLE delegation (
    delegator TEXT NOT NULL REFERENCES wallet (address),
    node TEXT NOT NULL,
    -- Store as string to preserve precision for U256
    amount TEXT NOT NULL,
    PRIMARY KEY (delegator, node)
);

CREATE INDEX delegation_by_node ON delegation (node);

-- Pending Withdrawals
CREATE TABLE pending_withdrawals (
    delegator TEXT NOT NULL REFERENCES wallet (address),
    node TEXT NOT NULL,
    withdrawal_type TEXT NOT NULL CHECK(withdrawal_type IN ('undelegation', 'exit')),
    amount TEXT NOT NULL,
    unlocks_at BIGINT NOT NULL,
    PRIMARY KEY (delegator, node, withdrawal_type)
);

-- Rewards
-- Stores total accrued rewards for each account
CREATE TABLE lifetime_rewards (
    address TEXT PRIMARY KEY,
    -- Store as string to preserve precision for U256
    amount TEXT NOT NULL
);

-- Active node set
-- Stores active nodes with participation metrics
CREATE TABLE active_node (
  address   VARCHAR PRIMARY KEY,
  idx       INT NOT NULL UNIQUE,
  votes     INT NOT NULL,
  proposals INT NOT NULL,
  slots     INT NOT NULL
);

-- Information about the latest processed Espresso block
CREATE TABLE espresso_block (
  -- The primary key row always has the value `1`, making this a singleton table.
  always_one  INT PRIMARY KEY,
  number      BIGINT NOT NULL,
  epoch       BIGINT NOT NULL,
  `timestamp` BIGINT NOT NULL
);
