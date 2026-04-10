/// Backward-compatible environment variable migration.
///
/// Maps deprecated `ESPRESSO_SEQUENCER_*` env vars to their new names.
/// If a new env var is already set, the old one is ignored. Otherwise the old value is
/// copied to the new name and a deprecation warning is emitted.
///
/// Call this function early in `main()`, before clap parsing.
pub fn migrate_legacy_env_vars() {
    const MAPPINGS: &[(&str, &str)] = &[
        // ── contracts/rust/deployment-info/src/addresses.rs ──
        // ESP token proxy contract address
        (
            "ESP_TOKEN_PROXY_ADDRESS",
            "ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS",
        ),
        // Fee contract proxy address
        (
            "ESPRESSO_FEE_CONTRACT_PROXY_ADDRESS",
            "ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS",
        ),
        // Reward claim proxy contract address
        (
            "ESPRESSO_REWARD_CLAIM_PROXY_ADDRESS",
            "ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS",
        ),
        // Operations timelock contract address
        (
            "ESPRESSO_OPS_TIMELOCK_ADDRESS",
            "ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS",
        ),
        // Safe exit timelock contract address
        (
            "ESPRESSO_SAFE_EXIT_TIMELOCK_ADDRESS",
            "ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS",
        ),
        // ── crates/builder/src/bin/permissionless-builder.rs ──
        // Peer nodes used to fetch missing state
        (
            "ESPRESSO_NODE_STATE_PEERS",
            "ESPRESSO_SEQUENCER_STATE_PEERS",
        ),
        // Espresso node API URL
        ("ESPRESSO_API_NODE_URL", "ESPRESSO_SEQUENCER_URL"),
        // ── crates/espresso/dev-node/src/main.rs ──
        // Max concurrent HTTP API connections
        (
            "ESPRESSO_NODE_API_MAX_CONNECTIONS",
            "ESPRESSO_SEQUENCER_MAX_CONNECTIONS",
        ),
        // Alternate account indices for multi-chain deployment
        (
            "ESPRESSO_DEPLOYER_ALT_INDICES",
            "ESPRESSO_SEQUENCER_DEPLOYER_ALT_INDICES",
        ),
        // Multisig admin address for contract operations
        (
            "ESPRESSO_ETH_MULTISIG_ADDRESS",
            "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS",
        ),
        // ── crates/espresso/dev-node/tests/dev_node_tests.rs ──
        // Port for the HTTP API server
        ("ESPRESSO_NODE_API_PORT", "ESPRESSO_SEQUENCER_API_PORT"),
        // Maximum database connections
        (
            "ESPRESSO_NODE_DATABASE_MAX_CONNECTIONS",
            "ESPRESSO_SEQUENCER_DATABASE_MAX_CONNECTIONS",
        ),
        // L1 JSON-RPC provider URL
        ("ESPRESSO_L1_PROVIDER", "ESPRESSO_SEQUENCER_L1_PROVIDER"),
        // Mnemonic for the deployer wallet
        ("ESPRESSO_ETH_MNEMONIC", "ESPRESSO_SEQUENCER_ETH_MNEMONIC"),
        // ── crates/espresso/node/src/api/options.rs ──
        // Peer URLs for fetching missing query service data
        ("ESPRESSO_NODE_API_PEERS", "ESPRESSO_SEQUENCER_API_PEERS"),
        // ── crates/espresso/node/src/bin/deploy.rs ──
        // Number of blocks per epoch for HotShot consensus
        (
            "ESPRESSO_NETWORK_BLOCKS_PER_EPOCH",
            "ESPRESSO_SEQUENCER_BLOCKS_PER_EPOCH",
        ),
        // Epoch start block number
        (
            "ESPRESSO_NETWORK_EPOCH_START_BLOCK",
            "ESPRESSO_SEQUENCER_EPOCH_START_BLOCK",
        ),
        // Multisig pauser address for emergency actions
        (
            "ESPRESSO_ETH_MULTISIG_PAUSER_ADDRESS",
            "ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS",
        ),
        // Stake table capacity for the prover circuit
        (
            "ESPRESSO_STAKE_TABLE_CAPACITY",
            "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY",
        ),
        // Exit escrow period for the stake table
        (
            "ESPRESSO_STAKE_TABLE_EXIT_ESCROW_PERIOD",
            "ESPRESSO_SEQUENCER_STAKE_TABLE_EXIT_ESCROW_PERIOD",
        ),
        // Address of the permissioned prover
        (
            "ESPRESSO_PERMISSIONED_PROVER",
            "ESPRESSO_SEQUENCER_PERMISSIONED_PROVER",
        ),
        // ── crates/espresso/node/src/bin/submit-transactions.rs ──
        // Comma-separated list of Espresso node API URLs
        ("ESPRESSO_API_NODE_URLS", "ESPRESSO_SEQUENCER_URLS"),
        // ── crates/espresso/node/src/keyset.rs ──
        // Path to file containing private keys
        ("ESPRESSO_NODE_KEY_FILE", "ESPRESSO_SEQUENCER_KEY_FILE"),
        // Index for generating multiple keysets from one mnemonic
        ("ESPRESSO_NODE_KEY_INDEX", "ESPRESSO_SEQUENCER_KEY_INDEX"),
        // Mnemonic phrase for key generation
        (
            "ESPRESSO_NODE_KEY_MNEMONIC",
            "ESPRESSO_SEQUENCER_KEY_MNEMONIC",
        ),
        // Private BLS staking key
        (
            "ESPRESSO_NODE_PRIVATE_STAKING_KEY",
            "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY",
        ),
        // Private Schnorr state signing key
        (
            "ESPRESSO_NODE_PRIVATE_STATE_KEY",
            "ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY",
        ),
        // Private x25519 encryption key
        (
            "ESPRESSO_NODE_PRIVATE_X25519_KEY",
            "ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY",
        ),
        // ── crates/espresso/node/src/options.rs ──
        // Base timeout for catchup requests to peers
        (
            "ESPRESSO_NODE_CATCHUP_BASE_TIMEOUT",
            "ESPRESSO_SEQUENCER_CATCHUP_BASE_TIMEOUT",
        ),
        // CDN endpoint for consensus messaging
        (
            "ESPRESSO_NODE_CDN_ENDPOINT",
            "ESPRESSO_SEQUENCER_CDN_ENDPOINT",
        ),
        // Bind address for cliquenet protocol
        (
            "ESPRESSO_NODE_CLIQUENET_BIND_ADDRESS",
            "ESPRESSO_SEQUENCER_CLIQUENET_BIND_ADDRESS",
        ),
        // Peer nodes used to fetch missing config
        (
            "ESPRESSO_NODE_CONFIG_PEERS",
            "ESPRESSO_SEQUENCER_CONFIG_PEERS",
        ),
        // Path to the genesis TOML file
        (
            "ESPRESSO_NODE_GENESIS_FILE",
            "ESPRESSO_SEQUENCER_GENESIS_FILE",
        ),
        // Node operator company name
        (
            "ESPRESSO_NODE_IDENTITY_COMPANY_NAME",
            "ESPRESSO_SEQUENCER_IDENTITY_COMPANY_NAME",
        ),
        // Node operator company website
        (
            "ESPRESSO_NODE_IDENTITY_COMPANY_WEBSITE",
            "ESPRESSO_SEQUENCER_IDENTITY_COMPANY_WEBSITE",
        ),
        // Node operator country code
        (
            "ESPRESSO_NODE_IDENTITY_COUNTRY_CODE",
            "ESPRESSO_SEQUENCER_IDENTITY_COUNTRY_CODE",
        ),
        // Node icon 14x14 1x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_14x14_1x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_1x",
        ),
        // Node icon 14x14 2x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_14x14_2x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_2x",
        ),
        // Node icon 14x14 3x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_14x14_3x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_3x",
        ),
        // Node icon 24x24 1x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_24x24_1x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_1x",
        ),
        // Node icon 24x24 2x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_24x24_2x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_2x",
        ),
        // Node icon 24x24 3x
        (
            "ESPRESSO_NODE_IDENTITY_ICON_24x24_3x",
            "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_3x",
        ),
        // Node operator latitude
        (
            "ESPRESSO_NODE_IDENTITY_LATITUDE",
            "ESPRESSO_SEQUENCER_IDENTITY_LATITUDE",
        ),
        // Node operator longitude
        (
            "ESPRESSO_NODE_IDENTITY_LONGITUDE",
            "ESPRESSO_SEQUENCER_IDENTITY_LONGITUDE",
        ),
        // Network type (e.g. local, testnet, mainnet)
        (
            "ESPRESSO_NODE_IDENTITY_NETWORK_TYPE",
            "ESPRESSO_SEQUENCER_IDENTITY_NETWORK_TYPE",
        ),
        // Node description
        (
            "ESPRESSO_NODE_IDENTITY_NODE_DESCRIPTION",
            "ESPRESSO_SEQUENCER_IDENTITY_NODE_DESCRIPTION",
        ),
        // Node display name
        (
            "ESPRESSO_NODE_IDENTITY_NODE_NAME",
            "ESPRESSO_SEQUENCER_IDENTITY_NODE_NAME",
        ),
        // Node type
        (
            "ESPRESSO_NODE_IDENTITY_NODE_TYPE",
            "ESPRESSO_SEQUENCER_IDENTITY_NODE_TYPE",
        ),
        // Node operating system
        (
            "ESPRESSO_NODE_IDENTITY_OPERATING_SYSTEM",
            "ESPRESSO_SEQUENCER_IDENTITY_OPERATING_SYSTEM",
        ),
        // Whether this node participates in DA
        ("ESPRESSO_NODE_IS_DA", "ESPRESSO_SEQUENCER_IS_DA"),
        // Public address advertised to libp2p peers
        (
            "ESPRESSO_NODE_LIBP2P_ADVERTISE_ADDRESS",
            "ESPRESSO_SEQUENCER_LIBP2P_ADVERTISE_ADDRESS",
        ),
        // Local address for the libp2p listener
        (
            "ESPRESSO_NODE_LIBP2P_BIND_ADDRESS",
            "ESPRESSO_SEQUENCER_LIBP2P_BIND_ADDRESS",
        ),
        // Bootstrap nodes for libp2p peer discovery
        (
            "ESPRESSO_NODE_LIBP2P_BOOTSTRAP_NODES",
            "ESPRESSO_SEQUENCER_LIBP2P_BOOTSTRAP_NODES",
        ),
        // Cache duration for libp2p message hashes
        (
            "ESPRESSO_NODE_LIBP2P_DUPLICATE_CACHE_TIME",
            "ESPRESSO_SEQUENCER_LIBP2P_DUPLICATE_CACHE_TIME",
        ),
        // TTL for libp2p fanout peers
        (
            "ESPRESSO_NODE_LIBP2P_FANOUT_TTL",
            "ESPRESSO_SEQUENCER_LIBP2P_FANOUT_TTL",
        ),
        // Flood publish messages in libp2p
        (
            "ESPRESSO_NODE_LIBP2P_FLOOD_PUBLISH",
            "ESPRESSO_SEQUENCER_LIBP2P_FLOOD_PUBLISH",
        ),
        // Gossip sub protocol factor
        (
            "ESPRESSO_NODE_LIBP2P_GOSSIP_FACTOR",
            "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_FACTOR",
        ),
        // Number of lazy gossip messages
        (
            "ESPRESSO_NODE_LIBP2P_GOSSIP_LAZY",
            "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_LAZY",
        ),
        // Gossip retransmission count
        (
            "ESPRESSO_NODE_LIBP2P_GOSSIP_RETRANSMISSION",
            "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_RETRANSMISSION",
        ),
        // Initial delay for libp2p heartbeat
        (
            "ESPRESSO_NODE_LIBP2P_HEARTBEAT_INITIAL_DELAY",
            "ESPRESSO_SEQUENCER_LIBP2P_HEARTBEAT_INITIAL_DELAY",
        ),
        // Interval between libp2p heartbeats
        (
            "ESPRESSO_NODE_LIBP2P_HEARTBEAT_INTERVAL",
            "ESPRESSO_SEQUENCER_LIBP2P_HEARTBEAT_INTERVAL",
        ),
        // Gossip history length
        (
            "ESPRESSO_NODE_LIBP2P_HISTORY_GOSSIP",
            "ESPRESSO_SEQUENCER_LIBP2P_HISTORY_GOSSIP",
        ),
        // History length for libp2p
        (
            "ESPRESSO_NODE_LIBP2P_HISTORY_LENGTH",
            "ESPRESSO_SEQUENCER_LIBP2P_HISTORY_LENGTH",
        ),
        // Max direct transmit message size
        (
            "ESPRESSO_NODE_LIBP2P_MAX_DIRECT_TRANSMIT_SIZE",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_DIRECT_TRANSMIT_SIZE",
        ),
        // Max gossip transmit message size
        (
            "ESPRESSO_NODE_LIBP2P_MAX_GOSSIP_TRANSMIT_SIZE",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_GOSSIP_TRANSMIT_SIZE",
        ),
        // Max IHAVE message length
        (
            "ESPRESSO_NODE_LIBP2P_MAX_IHAVE_LENGTH",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_IHAVE_LENGTH",
        ),
        // Max number of IHAVE messages
        (
            "ESPRESSO_NODE_LIBP2P_MAX_IHAVE_MESSAGES",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_IHAVE_MESSAGES",
        ),
        // Max IWANT follow-up time
        (
            "ESPRESSO_NODE_LIBP2P_MAX_IWANT_FOLLOWUP_TIME",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_IWANT_FOLLOWUP_TIME",
        ),
        // Max messages per libp2p RPC
        (
            "ESPRESSO_NODE_LIBP2P_MAX_MESSAGES_PER_RPC",
            "ESPRESSO_SEQUENCER_LIBP2P_MAX_MESSAGES_PER_RPC",
        ),
        // Target mesh network peer count
        (
            "ESPRESSO_NODE_LIBP2P_MESH_N",
            "ESPRESSO_SEQUENCER_LIBP2P_MESH_N",
        ),
        // Upper bound for mesh peers
        (
            "ESPRESSO_NODE_LIBP2P_MESH_N_HIGH",
            "ESPRESSO_SEQUENCER_LIBP2P_MESH_N_HIGH",
        ),
        // Lower bound for mesh peers
        (
            "ESPRESSO_NODE_LIBP2P_MESH_N_LOW",
            "ESPRESSO_SEQUENCER_LIBP2P_MESH_N_LOW",
        ),
        // Minimum outbound mesh peers
        (
            "ESPRESSO_NODE_LIBP2P_MESH_OUTBOUND_MIN",
            "ESPRESSO_SEQUENCER_LIBP2P_MESH_OUTBOUND_MIN",
        ),
        // Cache duration for published message IDs
        (
            "ESPRESSO_NODE_LIBP2P_PUBLISHED_MESSAGE_IDS_CACHE_TIME",
            "ESPRESSO_SEQUENCER_LIBP2P_PUBLISHED_MESSAGE_IDS_CACHE_TIME",
        ),
        // Orchestrator URL for consensus coordination
        (
            "ESPRESSO_NODE_ORCHESTRATOR_URL",
            "ESPRESSO_SEQUENCER_ORCHESTRATOR_URL",
        ),
        // Public API URL advertised to other nodes
        (
            "ESPRESSO_NODE_PUBLIC_API_URL",
            "ESPRESSO_SEQUENCER_PUBLIC_API_URL",
        ),
        // Builder URLs for submitting transactions
        ("ESPRESSO_BUILDER_URLS", "ESPRESSO_SEQUENCER_BUILDER_URLS"),
        // Remote providers fallback timeout
        (
            "ESPRESSO_NODE_LOCAL_CATCHUP_TIMEOUT",
            "ESPRESSO_SEQUENCER_LOCAL_CATCHUP_TIMEOUT",
        ),
        // ── crates/espresso/node/src/persistence/fs.rs ──
        // Number of consensus views to retain
        (
            "ESPRESSO_NODE_CONSENSUS_VIEW_RETENTION",
            "ESPRESSO_SEQUENCER_CONSENSUS_VIEW_RETENTION",
        ),
        // ── crates/espresso/node/src/persistence/sql.rs ──
        // Minimum delay between active fetches in a stream
        (
            "ESPRESSO_NODE_ACTIVE_FETCH_DELAY",
            "ESPRESSO_SEQUENCER_ACTIVE_FETCH_DELAY",
        ),
        // Duration to cache sync status results for.
        (
            "ESPRESSO_NODE_SYNC_STATUS_TTL",
            "ESPRESSO_SEQUENCER_SYNC_STATUS_TTL",
        ),
        // Disable pruning and reconstruct previously pruned data
        ("ESPRESSO_NODE_ARCHIVE", "ESPRESSO_SEQUENCER_ARCHIVE"),
        // Minimum delay between loading chunks in a stream
        (
            "ESPRESSO_NODE_CHUNK_FETCH_DELAY",
            "ESPRESSO_SEQUENCER_CHUNK_FETCH_DELAY",
        ),
        // Minimum retention for consensus storage
        (
            "ESPRESSO_NODE_CONSENSUS_STORAGE_MINIMUM_RETENTION",
            "ESPRESSO_SEQUENCER_CONSENSUS_STORAGE_MINIMUM_RETENTION",
        ),
        // Target retention for consensus storage
        (
            "ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_RETENTION",
            "ESPRESSO_SEQUENCER_CONSENSUS_STORAGE_TARGET_RETENTION",
        ),
        // Target disk usage for consensus storage
        (
            "ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE",
            "ESPRESSO_SEQUENCER_CONSENSUS_STORAGE_TARGET_USAGE",
        ),
        // Maximum lifetime of a database connection
        (
            "ESPRESSO_NODE_DATABASE_CONNECTION_TIMEOUT",
            "ESPRESSO_SEQUENCER_DATABASE_CONNECTION_TIMEOUT",
        ),
        // Maximum idle time of a database connection
        (
            "ESPRESSO_NODE_DATABASE_IDLE_CONNECTION_TIMEOUT",
            "ESPRESSO_SEQUENCER_DATABASE_IDLE_CONNECTION_TIMEOUT",
        ),
        // Minimum database connections
        (
            "ESPRESSO_NODE_DATABASE_MIN_CONNECTIONS",
            "ESPRESSO_SEQUENCER_DATABASE_MIN_CONNECTIONS",
        ),
        // Enable pruning with default parameters
        (
            "ESPRESSO_NODE_DATABASE_PRUNE",
            "ESPRESSO_SEQUENCER_DATABASE_PRUNE",
        ),
        // Max connections for query operations
        (
            "ESPRESSO_NODE_DATABASE_QUERY_MAX_CONNECTIONS",
            "ESPRESSO_SEQUENCER_DATABASE_QUERY_MAX_CONNECTIONS",
        ),
        // Min connections for query operations
        (
            "ESPRESSO_NODE_DATABASE_QUERY_MIN_CONNECTIONS",
            "ESPRESSO_SEQUENCER_DATABASE_QUERY_MIN_CONNECTIONS",
        ),
        // Threshold for logging slow SQL statements
        (
            "ESPRESSO_NODE_DATABASE_SLOW_STATEMENT_THRESHOLD",
            "ESPRESSO_SEQUENCER_DATABASE_SLOW_STATEMENT_THRESHOLD",
        ),
        // Max time for a single SQL statement before cancellation
        (
            "ESPRESSO_NODE_DATABASE_STATEMENT_TIMEOUT",
            "ESPRESSO_SEQUENCER_DATABASE_STATEMENT_TIMEOUT",
        ),
        // Disable the proactive scanner task
        (
            "ESPRESSO_NODE_DISABLE_PROACTIVE_FETCHING",
            "ESPRESSO_SEQUENCER_DISABLE_PROACTIVE_FETCHING",
        ),
        // Max concurrent fetch requests from peers
        (
            "ESPRESSO_NODE_FETCH_RATE_LIMIT",
            "ESPRESSO_SEQUENCER_FETCH_RATE_LIMIT",
        ),
        // Run in lightweight mode (no DA participation)
        (
            "ESPRESSO_NODE_LIGHTWEIGHT",
            "ESPRESSO_SEQUENCER_LIGHTWEIGHT",
        ),
        // Postgres database name
        (
            "ESPRESSO_NODE_POSTGRES_DATABASE",
            "ESPRESSO_SEQUENCER_POSTGRES_DATABASE",
        ),
        // Postgres server hostname
        (
            "ESPRESSO_NODE_POSTGRES_HOST",
            "ESPRESSO_SEQUENCER_POSTGRES_HOST",
        ),
        // Postgres password
        (
            "ESPRESSO_NODE_POSTGRES_PASSWORD",
            "ESPRESSO_SEQUENCER_POSTGRES_PASSWORD",
        ),
        // Postgres server port
        (
            "ESPRESSO_NODE_POSTGRES_PORT",
            "ESPRESSO_SEQUENCER_POSTGRES_PORT",
        ),
        // Use TLS for Postgres connection
        (
            "ESPRESSO_NODE_POSTGRES_USE_TLS",
            "ESPRESSO_SEQUENCER_POSTGRES_USE_TLS",
        ),
        // Postgres user
        (
            "ESPRESSO_NODE_POSTGRES_USER",
            "ESPRESSO_SEQUENCER_POSTGRES_USER",
        ),
        // Chunk size for proactive fetch scanning
        (
            "ESPRESSO_NODE_PROACTIVE_SCAN_CHUNK_SIZE",
            "ESPRESSO_SEQUENCER_PROACTIVE_SCAN_CHUNK_SIZE",
        ),
        // Interval between proactive fetch scans
        (
            "ESPRESSO_NODE_PROACTIVE_SCAN_INTERVAL",
            "ESPRESSO_SEQUENCER_PROACTIVE_SCAN_INTERVAL",
        ),
        // Batch size for pruning operations
        (
            "ESPRESSO_NODE_PRUNER_BATCH_SIZE",
            "ESPRESSO_SEQUENCER_PRUNER_BATCH_SIZE",
        ),
        // SQLite pages to vacuum per cycle
        (
            "ESPRESSO_NODE_PRUNER_INCREMENTAL_VACUUM_PAGES",
            "ESPRESSO_SEQUENCER_PRUNER_INCREMENTAL_VACUUM_PAGES",
        ),
        // Interval between pruning runs
        (
            "ESPRESSO_NODE_PRUNER_INTERVAL",
            "ESPRESSO_SEQUENCER_PRUNER_INTERVAL",
        ),
        // Max disk usage in basis points
        (
            "ESPRESSO_NODE_PRUNER_MAX_USAGE",
            "ESPRESSO_SEQUENCER_PRUNER_MAX_USAGE",
        ),
        // Minimum data retention period
        (
            "ESPRESSO_NODE_PRUNER_MINIMUM_RETENTION",
            "ESPRESSO_SEQUENCER_PRUNER_MINIMUM_RETENTION",
        ),
        // Pruning threshold in bytes
        (
            "ESPRESSO_NODE_PRUNER_PRUNING_THRESHOLD",
            "ESPRESSO_SEQUENCER_PRUNER_PRUNING_THRESHOLD",
        ),
        // Target data retention period
        (
            "ESPRESSO_NODE_PRUNER_TARGET_RETENTION",
            "ESPRESSO_SEQUENCER_PRUNER_TARGET_RETENTION",
        ),
        // Path for node storage (filesystem backend)
        (
            "ESPRESSO_NODE_STORAGE_PATH",
            "ESPRESSO_SEQUENCER_STORAGE_PATH",
        ),
        // Chunk size for sync status scanning
        (
            "ESPRESSO_NODE_SYNC_STATUS_CHUNK_SIZE",
            "ESPRESSO_SEQUENCER_SYNC_STATUS_CHUNK_SIZE",
        ),
        // ── crates/espresso/node/src/proposal_fetcher.rs ──
        // Timeout for proposal fetch requests
        (
            "ESPRESSO_NODE_PROPOSAL_FETCHER_FETCH_TIMEOUT",
            "ESPRESSO_SEQUENCER_PROPOSAL_FETCHER_FETCH_TIMEOUT",
        ),
        // Number of proposal fetcher workers
        (
            "ESPRESSO_NODE_PROPOSAL_FETCHER_NUM_WORKERS",
            "ESPRESSO_SEQUENCER_PROPOSAL_FETCHER_NUM_WORKERS",
        ),
        // ── crates/espresso/types/src/v0/v0_4/state.rs ──
        // Reward merkle tree concurrent update permits
        (
            "ESPRESSO_NODE_REWARD_MERKLE_TREE_PERMITS",
            "ESPRESSO_SEQUENCER_REWARD_MERKLE_TREE_PERMITS",
        ),
        // ── crates/espresso/types/src/v0/utils.rs ──
        // Disable catchup retries after first failure
        (
            "ESPRESSO_NODE_CATCHUP_BACKOFF_DISABLE",
            "ESPRESSO_SEQUENCER_CATCHUP_BACKOFF_DISABLE",
        ),
        // Exponential backoff factor for catchup retries
        (
            "ESPRESSO_NODE_CATCHUP_BACKOFF_FACTOR",
            "ESPRESSO_SEQUENCER_CATCHUP_BACKOFF_FACTOR",
        ),
        // Jitter for catchup retry backoff
        (
            "ESPRESSO_NODE_CATCHUP_BACKOFF_JITTER",
            "ESPRESSO_SEQUENCER_CATCHUP_BACKOFF_JITTER",
        ),
        // Base delay between catchup retries
        (
            "ESPRESSO_NODE_CATCHUP_BASE_RETRY_DELAY",
            "ESPRESSO_SEQUENCER_CATCHUP_BASE_RETRY_DELAY",
        ),
        // Maximum delay between catchup retries
        (
            "ESPRESSO_NODE_CATCHUP_MAX_RETRY_DELAY",
            "ESPRESSO_SEQUENCER_CATCHUP_MAX_RETRY_DELAY",
        ),
        // ── crates/espresso/types/src/v0/v0_1/l1.rs ──
        // Interval for polling L1 stake table updates
        (
            "ESPRESSO_NODE_L1_STAKE_TABLE_UPDATE_INTERVAL",
            "ESPRESSO_SEQUENCER_L1_STAKE_TABLE_UPDATE_INTERVAL",
        ),
        // Number of L1 blocks to cache
        (
            "ESPRESSO_L1_BLOCKS_CACHE_SIZE",
            "ESPRESSO_SEQUENCER_L1_BLOCKS_CACHE_SIZE",
        ),
        // Consecutive L1 failures before failover
        (
            "ESPRESSO_L1_CONSECUTIVE_FAILURE_TOLERANCE",
            "ESPRESSO_SEQUENCER_L1_CONSECUTIVE_FAILURE_TOLERANCE",
        ),
        // Channel capacity for L1 event processing
        (
            "ESPRESSO_L1_EVENTS_CHANNEL_CAPACITY",
            "ESPRESSO_SEQUENCER_L1_EVENTS_CHANNEL_CAPACITY",
        ),
        // Max block range per L1 event query
        (
            "ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE",
            "ESPRESSO_SEQUENCER_L1_EVENTS_MAX_BLOCK_RANGE",
        ),
        // Max retry duration for L1 event fetching
        (
            "ESPRESSO_L1_EVENTS_MAX_RETRY_DURATION",
            "ESPRESSO_SEQUENCER_L1_EVENTS_MAX_RETRY_DURATION",
        ),
        // Revert to primary L1 provider after failover
        (
            "ESPRESSO_L1_FAILOVER_REVERT",
            "ESPRESSO_SEQUENCER_L1_FAILOVER_REVERT",
        ),
        // Safety margin for L1 finalized block lookback
        (
            "ESPRESSO_L1_FINALIZED_SAFETY_MARGIN",
            "ESPRESSO_SEQUENCER_L1_FINALIZED_SAFETY_MARGIN",
        ),
        // Frequent L1 failures before failover
        (
            "ESPRESSO_L1_FREQUENT_FAILURE_TOLERANCE",
            "ESPRESSO_SEQUENCER_L1_FREQUENT_FAILURE_TOLERANCE",
        ),
        // L1 polling interval
        (
            "ESPRESSO_L1_POLLING_INTERVAL",
            "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        ),
        // Delay between L1 RPC requests for rate limiting
        (
            "ESPRESSO_L1_RATE_LIMIT_DELAY",
            "ESPRESSO_SEQUENCER_L1_RATE_LIMIT_DELAY",
        ),
        // Delay before retrying a failed L1 request
        (
            "ESPRESSO_L1_RETRY_DELAY",
            "ESPRESSO_SEQUENCER_L1_RETRY_DELAY",
        ),
        // Timeout for L1 WebSocket subscriptions
        (
            "ESPRESSO_L1_SUBSCRIPTION_TIMEOUT",
            "ESPRESSO_SEQUENCER_L1_SUBSCRIPTION_TIMEOUT",
        ),
        // L1 WebSocket provider URL for subscriptions
        (
            "ESPRESSO_L1_WS_PROVIDER",
            "ESPRESSO_SEQUENCER_L1_WS_PROVIDER",
        ),
        // ── hotshot-state-prover/src/bin/state-prover.rs ──
        // Light client proxy contract address
        (
            "ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS",
            "ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS",
        ),
        // Account index for the state prover wallet
        (
            "ESPRESSO_STATE_PROVER_ACCOUNT_INDEX",
            "ESPRESSO_SEQUENCER_STATE_PROVER_ACCOUNT_INDEX",
        ),
        // ── staking-cli/src/lib.rs ──
        // Stake table proxy contract address
        (
            "ESPRESSO_STAKE_TABLE_PROXY_ADDRESS",
            "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS",
        ),
        // ── unknown ──
        // Light client contract address
        (
            "ESPRESSO_LIGHT_CLIENT_ADDRESS",
            "ESPRESSO_SEQUENCER_LIGHT_CLIENT_ADDRESS",
        ),
        // Light client V2 contract address
        (
            "ESPRESSO_LIGHT_CLIENT_V2_ADDRESS",
            "ESPRESSO_SEQUENCER_LIGHT_CLIENT_V2_ADDRESS",
        ),
        // Light client V3 contract address
        (
            "ESPRESSO_LIGHT_CLIENT_V3_ADDRESS",
            "ESPRESSO_SEQUENCER_LIGHT_CLIENT_V3_ADDRESS",
        ),
        // Stake table contract address
        (
            "ESPRESSO_STAKE_TABLE_ADDRESS",
            "ESPRESSO_SEQUENCER_STAKE_TABLE_ADDRESS",
        ),
        // Stake table V2 contract address
        (
            "ESPRESSO_STAKE_TABLE_V2_ADDRESS",
            "ESPRESSO_SEQUENCER_STAKE_TABLE_V2_ADDRESS",
        ),
        // ESP token contract address
        ("ESP_TOKEN_ADDRESS", "ESPRESSO_SEQUENCER_ESP_TOKEN_ADDRESS"),
        // ESP token V2 contract address
        (
            "ESP_TOKEN_V2_ADDRESS",
            "ESPRESSO_SEQUENCER_ESP_TOKEN_V2_ADDRESS",
        ),
        // Fee contract address
        (
            "ESPRESSO_FEE_CONTRACT_ADDRESS",
            "ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS",
        ),
        // Reward claim contract address
        (
            "ESPRESSO_REWARD_CLAIM_ADDRESS",
            "ESPRESSO_SEQUENCER_REWARD_CLAIM_ADDRESS",
        ),
        // PlonkVerifier contract address
        (
            "ESPRESSO_PLONK_VERIFIER_ADDRESS",
            "ESPRESSO_SEQUENCER_PLONK_VERIFIER_ADDRESS",
        ),
        // PlonkVerifier V2 contract address
        (
            "ESPRESSO_PLONK_VERIFIER_V2_ADDRESS",
            "ESPRESSO_SEQUENCER_PLONK_VERIFIER_V2_ADDRESS",
        ),
        // PlonkVerifier V3 contract address
        (
            "ESPRESSO_PLONK_VERIFIER_V3_ADDRESS",
            "ESPRESSO_SEQUENCER_PLONK_VERIFIER_V3_ADDRESS",
        ),
    ];

    for &(new, old) in MAPPINGS {
        if std::env::var(new).is_err()
            && let Ok(val) = std::env::var(old)
        {
            // Log to stderr immediately (tracing may not be initialized yet).
            eprintln!("WARNING: {old} is deprecated, use {new} instead");
            // Also log via tracing for structured logging (no-op if tracing is not yet initialized).
            tracing::warn!(%old, %new, "deprecated env var detected, mapping to new name");
            // SAFETY: called once at startup before spawning threads.
            unsafe { std::env::set_var(new, val) };
        }
    }
}
