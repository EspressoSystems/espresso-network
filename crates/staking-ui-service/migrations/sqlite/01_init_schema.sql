 
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

-- Delegations, pending withdrawals, and exits
CREATE TABLE delegation (
    delegator TEXT NOT NULL REFERENCES wallet (address),
    node TEXT NOT NULL,
    -- Store as string to preserve precision for U256
    amount TEXT NOT NULL,
    unlocks_at BIGINT NOT NULL,
    -- Store as string to preserve precision for U256
    withdrawal_amount TEXT NOT NULL,
    PRIMARY KEY (delegator, node)
);

CREATE INDEX delegation_by_node ON delegation (node);
CREATE INDEX delegation_by_status ON delegation (delegator, unlocks_at, withdrawal_amount);

-- Rewards
-- Stores total accrued rewards for each account
CREATE TABLE lifetime_rewards (
    address TEXT PRIMARY KEY,
    -- Store as string to preserve precision for U256
    amount TEXT NOT NULL
);
