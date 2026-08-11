//! Probes for the espresso node API.
//!
//! Node 0 carries the full matrix. Nodes 1 to 4 run different serve modes (explorer, bare,
//! filesystem storage, sqlite), so each gets the routes that distinguish its mode plus its
//! OpenAPI document, which pins the whole route inventory that mode exposes.
//!
//! Paths use `{placeholder}` params resolved once per run (see `Params` in the harness), so no
//! hash or height is hard-coded here.

use crate::common::api_snapshot::{Norm, Probe, Service, Service::*, WsProbe};

/// The genesis header records the L1 block the demo's anvil happened to start from, whose hash and
/// timestamp differ every run, and every commitment taken over that header differs with it. Masking
/// these leaves the rest of the header pinned exactly: the merkle roots, the payload commitment, the
/// chain config, and the header's own `timestamp`.
const PER_RUN: &[&str] = &[
    "l1_finalized.*",
    "hash",
    "block_hash",
    "leaf_commit",
    "vote_commitment",
];

/// The light client serves a leaf together with the quorum certificate that decided it. Which
/// validators' votes made it into that certificate differs between runs, so its aggregate signature
/// and signer bitvector move too. The plain availability routes do not need this: the certificate
/// they return for genesis carries no signatures.
const PER_RUN_LIGHT_CLIENT: &[&str] = &[
    "l1_finalized.*",
    "hash",
    "block_hash",
    "leaf_commit",
    "vote_commitment",
    "signatures",
];

/// A response determined by the genesis file, pinned exactly apart from [`PER_RUN`].
const fn genesis(service: Service, name: &'static str, path: &'static str) -> Probe {
    Probe::new(service, name, path).mask_fields(PER_RUN)
}

const fn genesis_lc(service: Service, name: &'static str, path: &'static str) -> Probe {
    Probe::new(service, name, path).mask_fields(PER_RUN_LIGHT_CLIENT)
}

const fn genesis_ws(service: Service, name: &'static str, path: &'static str) -> WsProbe {
    WsProbe::new(service, name, path).mask_fields(PER_RUN)
}

/// Genesis-determined responses, pinned value for value apart from [`PER_RUN`]. A diff here means
/// the encoding or the genesis changed.
pub const AVAILABILITY_GENESIS: &[Probe] = &[
    genesis(Node0, "v1_availability_leaf_0", "/v1/availability/leaf/0"),
    genesis(
        Node0,
        "v1_availability_leaf_by_hash",
        "/v1/availability/leaf/hash/{leaf_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_leaf_range",
        "/v1/availability/leaf/0/1",
    ),
    genesis(
        Node0,
        "v1_availability_header_0",
        "/v1/availability/header/0",
    ),
    genesis(
        Node0,
        "v1_availability_header_by_hash",
        "/v1/availability/header/hash/{block_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_header_by_payload_hash",
        "/v1/availability/header/payload-hash/{payload_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_header_range",
        "/v1/availability/header/0/1",
    ),
    genesis(Node0, "v1_availability_block_0", "/v1/availability/block/0"),
    genesis(
        Node0,
        "v1_availability_block_by_hash",
        "/v1/availability/block/hash/{block_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_block_by_payload_hash",
        "/v1/availability/block/payload-hash/{payload_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_block_range",
        "/v1/availability/block/0/1",
    ),
    genesis(
        Node0,
        "v1_availability_payload_0",
        "/v1/availability/payload/0",
    ),
    genesis(
        Node0,
        "v1_availability_payload_by_hash",
        "/v1/availability/payload/hash/{payload_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_payload_by_block_hash",
        "/v1/availability/payload/block-hash/{block_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_payload_range",
        "/v1/availability/payload/0/1",
    ),
    genesis(
        Node0,
        "v1_availability_vid_common_0",
        "/v1/availability/vid/common/0",
    ),
    genesis(
        Node0,
        "v1_availability_vid_common_by_hash",
        "/v1/availability/vid/common/hash/{block_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_vid_common_by_payload_hash",
        "/v1/availability/vid/common/payload-hash/{payload_hash}",
    ),
    genesis(
        Node0,
        "v1_availability_vid_common_range",
        "/v1/availability/vid/common/0/1",
    ),
    genesis(
        Node0,
        "v1_availability_block_summary_0",
        "/v1/availability/block/summary/0",
    ),
    genesis(
        Node0,
        "v1_availability_block_summaries_range",
        "/v1/availability/block/summaries/0/1",
    ),
    genesis(Node0, "v1_availability_limits", "/v1/availability/limits"),
];

/// Transaction lookups, keyed on a transaction discovered in the running chain. Bodies carry the
/// load generator's random payloads, so only their shape is pinned.
pub const AVAILABILITY_TRANSACTIONS: &[Probe] = &[
    Probe::shape(
        Node0,
        "v1_availability_transaction_by_position",
        "/v1/availability/transaction/{tx_height}/{tx_index}",
    ),
    Probe::shape(
        Node0,
        "v1_availability_transaction_by_hash",
        "/v1/availability/transaction/hash/{tx_hash}",
    ),
    Probe::shape(
        Node0,
        "v1_availability_transaction_by_position_noproof",
        "/v1/availability/transaction/{tx_height}/{tx_index}/noproof",
    ),
    Probe::shape(
        Node0,
        "v1_availability_transaction_by_hash_noproof",
        "/v1/availability/transaction/hash/{tx_hash}/noproof",
    ),
    Probe::shape(
        Node0,
        "v1_availability_transaction_proof_by_position",
        "/v1/availability/transaction/{tx_height}/{tx_index}/proof",
    ),
    Probe::shape(
        Node0,
        "v1_availability_transaction_proof_by_hash",
        "/v1/availability/transaction/hash/{tx_hash}/proof",
    ),
    Probe::shape(
        Node0,
        "v1_availability_namespace_proof_by_height",
        "/v1/availability/block/{tx_height}/namespace/{ns}",
    ),
    Probe::shape(
        Node0,
        "v1_availability_namespace_proof_range",
        "/v1/availability/block/{tx_height}/{tx_height_next}/namespace/{ns}",
    ),
];

/// Counters and views over the running chain: structure is pinned, values are not.
pub const NODE_MODULE: &[Probe] = &[
    Probe::shape(Node0, "v1_node_block_height", "/v1/node/block-height"),
    Probe::shape(
        Node0,
        "v1_node_transactions_count",
        "/v1/node/transactions/count",
    ),
    Probe::shape(
        Node0,
        "v1_node_transactions_count_to",
        "/v1/node/transactions/count/1",
    ),
    Probe::shape(
        Node0,
        "v1_node_transactions_count_from_to",
        "/v1/node/transactions/count/0/1",
    ),
    Probe::shape(
        Node0,
        "v1_node_transactions_count_ns",
        "/v1/node/transactions/count/namespace/{ns}",
    ),
    Probe::shape(
        Node0,
        "v1_node_transactions_count_ns_to",
        "/v1/node/transactions/count/namespace/{ns}/{head}",
    ),
    Probe::shape(
        Node0,
        "v1_node_transactions_count_ns_from_to",
        "/v1/node/transactions/count/namespace/{ns}/0/{head}",
    ),
    Probe::shape(Node0, "v1_node_payloads_size", "/v1/node/payloads/size"),
    Probe::shape(
        Node0,
        "v1_node_payloads_size_to",
        "/v1/node/payloads/size/1",
    ),
    Probe::shape(
        Node0,
        "v1_node_payloads_size_from_to",
        "/v1/node/payloads/size/0/1",
    ),
    Probe::shape(
        Node0,
        "v1_node_payloads_total_size",
        "/v1/node/payloads/total-size",
    ),
    Probe::shape(
        Node0,
        "v1_node_payloads_size_ns",
        "/v1/node/payloads/size/namespace/{ns}",
    ),
    Probe::shape(
        Node0,
        "v1_node_payloads_size_ns_to",
        "/v1/node/payloads/size/namespace/{ns}/{head}",
    ),
    Probe::shape(
        Node0,
        "v1_node_payloads_size_ns_from_to",
        "/v1/node/payloads/size/namespace/{ns}/0/{head}",
    ),
    Probe::shape(Node0, "v1_node_vid_share_0", "/v1/node/vid/share/0"),
    Probe::shape(
        Node0,
        "v1_node_vid_share_by_hash",
        "/v1/node/vid/share/hash/{block_hash}",
    ),
    Probe::shape(
        Node0,
        "v1_node_vid_share_by_payload_hash",
        "/v1/node/vid/share/payload-hash/{payload_hash}",
    ),
    Probe::shape(Node0, "v1_node_sync_status", "/v1/node/sync-status"),
    Probe::shape(Node0, "v1_node_limits", "/v1/node/limits"),
    Probe::shape(Node0, "v1_node_oldest_block", "/v1/node/oldest-block"),
    Probe::shape(Node0, "v1_node_oldest_leaf", "/v1/node/oldest-leaf"),
    Probe::shape(
        Node0,
        "v1_node_header_window_from_height",
        "/v1/node/header/window/from/0/{head}",
    ),
    Probe::shape(
        Node0,
        "v1_node_stake_table_current",
        "/v1/node/stake-table/current",
    ),
    Probe::shape(
        Node0,
        "v1_node_da_stake_table_current",
        "/v1/node/da-stake-table/current",
    ),
    Probe::shape(Node0, "v1_node_block_reward", "/v1/node/block-reward"),
];

/// Endpoints keyed on an epoch, plus the reward accounting that proof of stake introduced. The
/// epoch is resolved to one the node will serve, so this set works on any versioned genesis that
/// has epochs at all.
pub const EPOCHS_AND_REWARDS: &[Probe] = &[
    Probe::shape(
        Node0,
        "v1_node_stake_table_epoch",
        "/v1/node/stake-table/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_node_da_stake_table_epoch",
        "/v1/node/da-stake-table/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_node_validators_epoch",
        "/v1/node/validators/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_node_block_reward_epoch",
        "/v1/node/block-reward/epoch/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_availability_state_cert",
        "/v1/availability/state-cert/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_availability_state_cert_v2",
        "/v1/availability/state-cert-v2/{epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_light_client_stake_table_epoch",
        "/v1/light-client/stake-table/{lc_epoch}",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_v2_balance",
        "/v1/reward-state-v2/reward-balance/{head}/{validator_address}",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_v2_balance_latest",
        "/v1/reward-state-v2/reward-balance/latest/{validator_address}",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_v2_proof",
        "/v1/reward-state-v2/proof/{head}/{validator_address}",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_v2_claim_input",
        "/v1/reward-state-v2/reward-claim-input/{head}/{validator_address}",
    ),
];

/// Merklized state, status, config and token modules.
pub const STATE_AND_STATUS: &[Probe] = &[
    Probe::shape(Node0, "v1_status_block_height", "/v1/status/block-height"),
    Probe::shape(Node0, "v1_status_success_rate", "/v1/status/success-rate"),
    Probe::shape(
        Node0,
        "v1_status_time_since_last_decide",
        "/v1/status/time-since-last-decide",
    ),
    Probe::new(Node0, "v1_status_metrics", "/v1/status/metrics").norm(Norm::MetricNames),
    Probe::shape(
        Node0,
        "v1_block_state_block_height",
        "/v1/block-state/block-height",
    ),
    Probe::shape(
        Node0,
        "v1_block_state_path_by_height",
        "/v1/block-state/1/0",
    ),
    Probe::shape(
        Node0,
        "v1_fee_state_block_height",
        "/v1/fee-state/block-height",
    ),
    Probe::shape(
        Node0,
        "v1_fee_state_balance_latest",
        "/v1/fee-state/fee-balance/latest/{builder_address}",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_block_height",
        "/v1/reward-state/block-height",
    ),
    Probe::shape(
        Node0,
        "v1_reward_state_v2_block_height",
        "/v1/reward-state-v2/block-height",
    ),
    // The hotshot config is per-run only in its network addresses and keys, which the demo pins
    // in .env, so the whole document is stable.
    Probe::shape(Node0, "v1_config_hotshot", "/v1/config/hotshot"),
    Probe::shape(Node0, "v1_config_env", "/v1/config/env"),
    Probe::shape(Node0, "v1_config_runtime", "/v1/config/runtime"),
    Probe::shape(
        Node0,
        "v1_token_total_minted_supply",
        "/v1/token/total-minted-supply",
    ),
    Probe::shape(
        Node0,
        "v1_token_circulating_supply",
        "/v1/token/circulating-supply",
    ),
    Probe::shape(
        Node0,
        "v1_token_circulating_supply_ethereum",
        "/v1/token/circulating-supply-ethereum",
    ),
    Probe::shape(
        Node0,
        "v1_token_total_issued_supply",
        "/v1/token/total-issued-supply",
    ),
    Probe::shape(
        Node0,
        "v1_token_total_reward_distributed",
        "/v1/token/total-reward-distributed",
    ),
    Probe::shape(Node0, "v1_database_table_sizes", "/v1/database/table-sizes"),
    Probe::shape(
        Node0,
        "v1_database_migration_status",
        "/v1/database/migration-status",
    ),
    Probe::shape(
        Node0,
        "v1_state_signature_block",
        "/v1/state-signature/block/{signed_height}",
    ),
];

/// Catchup serves the state a restarting node needs. Heights and views come from a decided leaf.
pub const CATCHUP: &[Probe] = &[
    Probe::shape(
        Node0,
        "v1_catchup_account",
        "/v1/catchup/{catchup_height}/{catchup_view}/account/{builder_address}",
    ),
    // `catchup/chain-config/{commitment}` is not probed: the commitment is only recoverable by
    // committing a ChainConfig locally, which would duplicate the node's own logic in a test.
    Probe::shape(
        Node0,
        "v1_catchup_leafchain",
        "/v1/catchup/{catchup_height}/leafchain",
    ),
];

/// The light client query service, served by node 0 only.
pub const LIGHT_CLIENT: &[Probe] = &[
    genesis_lc(Node0, "v1_light_client_leaf_0", "/v1/light-client/leaf/0"),
    genesis_lc(
        Node0,
        "v1_light_client_leaf_by_hash",
        "/v1/light-client/leaf/hash/{leaf_hash}",
    ),
    genesis_lc(
        Node0,
        "v1_light_client_leaf_by_block_hash",
        "/v1/light-client/leaf/block-hash/{block_hash}",
    ),
    genesis_lc(
        Node0,
        "v1_light_client_leaf_by_payload_hash",
        "/v1/light-client/leaf/payload-hash/{payload_hash}",
    ),
    genesis_lc(
        Node0,
        "v1_light_client_payload_0",
        "/v1/light-client/payload/0",
    ),
    genesis_lc(
        Node0,
        "v1_light_client_payload_range",
        "/v1/light-client/payload/0/1",
    ),
];

/// The explorer, served by node 1 only. Every body carries live aggregates.
pub const EXPLORER: &[Probe] = &[
    Probe::shape(Node1, "v1_explorer_block_detail", "/v1/explorer/block/0"),
    Probe::shape(
        Node1,
        "v1_explorer_block_detail_by_hash",
        "/v1/explorer/block/hash/{block_hash}",
    ),
    Probe::shape(
        Node1,
        "v1_explorer_blocks_latest",
        "/v1/explorer/blocks/latest/1",
    ),
    Probe::shape(
        Node1,
        "v1_explorer_transactions_latest",
        "/v1/explorer/transactions/latest/1",
    ),
    Probe::shape(
        Node1,
        "v1_explorer_summary",
        "/v1/explorer/explorer-summary",
    ),
];

/// The v2 API is proto-backed and has no `/v0` alias.
pub const V2: &[Probe] = &[
    Probe::shape(Node0, "v2_openapi", "/v2/docs/openapi.json").no_aliases(),
    Probe::shape(
        Node0,
        "v2_rewards_balance",
        "/v2/rewards/balance?address={validator_address}",
    )
    .no_aliases(),
];

/// Documents and the routes tide-disco used to provide for free. The OpenAPI document is the
/// highest-value snapshot in the suite: adding or removing any v1 route changes it.
///
/// `/healthcheck` is not snapshotted anywhere. It is still used to wait for services to come up.
pub const TOP_LEVEL: &[Probe] = &[
    Probe::new(Node0, "v1_openapi", "/v1/docs/openapi.json").no_aliases(),
    Probe::new(Node0, "version", "/version").no_aliases(),
];

/// One OpenAPI document per node pins which modules that serve mode exposes.
pub const PER_NODE: &[Probe] = &[
    Probe::new(Node1, "v1_openapi", "/v1/docs/openapi.json").no_aliases(),
    Probe::new(Node2, "v1_openapi", "/v1/docs/openapi.json").no_aliases(),
    Probe::new(Node3, "v1_openapi", "/v1/docs/openapi.json").no_aliases(),
    Probe::new(Node4, "v1_openapi", "/v1/docs/openapi.json").no_aliases(),
    // Nodes 3 and 4 use filesystem and sqlite storage: the same query routes must serve the same
    // genesis bodies as node 0's postgres.
    genesis(Node3, "v1_availability_leaf_0", "/v1/availability/leaf/0"),
    genesis(
        Node3,
        "v1_availability_header_0",
        "/v1/availability/header/0",
    ),
    genesis(Node4, "v1_availability_leaf_0", "/v1/availability/leaf/0"),
    genesis(
        Node4,
        "v1_availability_header_0",
        "/v1/availability/header/0",
    ),
    Probe::shape(Node4, "v1_node_block_height", "/v1/node/block-height"),
];

/// Streams. The first message of a stream started at genesis is the genesis object, so it is
/// pinned exactly and must agree with the corresponding one-shot route.
pub const STREAMS: &[WsProbe] = &[
    genesis_ws(
        Node0,
        "v1_availability_stream_leaves",
        "/v1/availability/stream/leaves/0",
    ),
    genesis_ws(
        Node0,
        "v1_availability_stream_headers",
        "/v1/availability/stream/headers/0",
    ),
    genesis_ws(
        Node0,
        "v1_availability_stream_blocks",
        "/v1/availability/stream/blocks/0",
    ),
    genesis_ws(
        Node0,
        "v1_availability_stream_payloads",
        "/v1/availability/stream/payloads/0",
    ),
    genesis_ws(
        Node0,
        "v1_availability_stream_vid_common",
        "/v1/availability/stream/vid/common/0",
    ),
    WsProbe::shape(
        Node0,
        "v1_availability_stream_transactions",
        "/v1/availability/stream/transactions/{tx_height}",
    ),
];
