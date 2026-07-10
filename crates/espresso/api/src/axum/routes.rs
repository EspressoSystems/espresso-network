/// Generate a path-builder function from a route constant.
///
/// Substitutes each `{placeholder}` segment with the corresponding argument
/// (formatted via `Display`). Used to keep request URLs in sync with the
/// route definitions registered with the Axum router.
macro_rules! path_fn {
    ($name:ident, $route:expr $(,)?) => {
        pub fn $name() -> String {
            ::std::string::String::from($route)
        }
    };
    ($name:ident, $route:expr, $($placeholder:literal => $param:ident),+ $(,)?) => {
        pub fn $name($($param: impl ::std::fmt::Display),+) -> String {
            let mut path = ::std::string::String::from($route);
            $(
                path = path.replace(
                    concat!("{", $placeholder, "}"),
                    &$param.to_string(),
                );
            )+
            path
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// HTTP path for Axum handler (e.g., "/v2/rewards/balance/{height}/{address}")
    pub http: &'static str,
    /// gRPC path for Tonic service (e.g., "/espresso.api.v2.RewardService/GetRewardBalance")
    pub grpc: &'static str,
    /// OpenAPI description for the endpoint
    pub description: &'static str,
    /// OpenAPI tag grouping for the endpoint
    pub tag: &'static str,
}

pub mod v1 {
    pub const REWARD_CLAIM_INPUT_ROUTE: &str =
        "/v1/reward-state-v2/reward-claim-input/{height}/{address}";
    pub const REWARD_BALANCE_ROUTE: &str = "/v1/reward-state-v2/reward-balance/{height}/{address}";
    pub const LATEST_REWARD_BALANCE_ROUTE: &str =
        "/v1/reward-state-v2/reward-balance/latest/{address}";
    pub const REWARD_ACCOUNT_PROOF_ROUTE: &str = "/v1/reward-state-v2/proof/{height}/{address}";
    pub const LATEST_REWARD_ACCOUNT_PROOF_ROUTE: &str =
        "/v1/reward-state-v2/proof/latest/{address}";
    pub const REWARD_AMOUNTS_ROUTE: &str =
        "/v1/reward-state-v2/reward-amounts/{height}/{offset}/{limit}";
    pub const REWARD_MERKLE_TREE_V2_ROUTE: &str =
        "/v1/reward-state-v2/reward-merkle-tree-v2/{height}";

    pub const NAMESPACE_PROOF_BY_HEIGHT_ROUTE: &str =
        "/v1/availability/block/{height}/namespace/{namespace}";
    pub const NAMESPACE_PROOF_BY_HASH_ROUTE: &str =
        "/v1/availability/block/hash/{hash}/namespace/{namespace}";
    pub const NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/block/payload-hash/{payload-hash}/namespace/{namespace}";
    pub const NAMESPACE_PROOF_RANGE_ROUTE: &str =
        "/v1/availability/block/{from}/{until}/namespace/{namespace}";
    pub const INCORRECT_ENCODING_PROOF_ROUTE: &str =
        "/v1/availability/incorrect-encoding-proof/{block_number}/{namespace}";

    pub const STATE_CERT_V1_ROUTE: &str = "/v1/availability/state-cert/{epoch}";
    pub const STATE_CERT_V2_ROUTE: &str = "/v1/availability/state-cert-v2/{epoch}";

    pub const LEAF_BY_HEIGHT_ROUTE: &str = "/v1/availability/leaf/{height}";
    pub const LEAF_BY_HASH_ROUTE: &str = "/v1/availability/leaf/hash/{hash}";
    pub const LEAF_RANGE_ROUTE: &str = "/v1/availability/leaf/{from}/{until}";

    pub const HEADER_BY_HEIGHT_ROUTE: &str = "/v1/availability/header/{height}";
    pub const HEADER_BY_HASH_ROUTE: &str = "/v1/availability/header/hash/{hash}";
    pub const HEADER_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/header/payload-hash/{payload_hash}";
    pub const HEADER_RANGE_ROUTE: &str = "/v1/availability/header/{from}/{until}";

    pub const BLOCK_BY_HEIGHT_ROUTE: &str = "/v1/availability/block/{height}";
    pub const BLOCK_BY_HASH_ROUTE: &str = "/v1/availability/block/hash/{hash}";
    pub const BLOCK_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/block/payload-hash/{payload_hash}";
    pub const BLOCK_RANGE_ROUTE: &str = "/v1/availability/block/{from}/{until}";

    pub const PAYLOAD_BY_HEIGHT_ROUTE: &str = "/v1/availability/payload/{height}";
    pub const PAYLOAD_BY_HASH_ROUTE: &str = "/v1/availability/payload/hash/{hash}";
    pub const PAYLOAD_BY_BLOCK_HASH_ROUTE: &str =
        "/v1/availability/payload/block-hash/{block_hash}";
    pub const PAYLOAD_RANGE_ROUTE: &str = "/v1/availability/payload/{from}/{until}";

    pub const VID_COMMON_BY_HEIGHT_ROUTE: &str = "/v1/availability/vid/common/{height}";
    pub const VID_COMMON_BY_HASH_ROUTE: &str = "/v1/availability/vid/common/hash/{hash}";
    pub const VID_COMMON_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/vid/common/payload-hash/{payload_hash}";
    pub const VID_COMMON_RANGE_ROUTE: &str = "/v1/availability/vid/common/{from}/{until}";

    pub const TRANSACTION_BY_POSITION_NOPROOF_ROUTE: &str =
        "/v1/availability/transaction/{height}/{index}/noproof";
    pub const TRANSACTION_BY_HASH_NOPROOF_ROUTE: &str =
        "/v1/availability/transaction/hash/{hash}/noproof";
    pub const TRANSACTION_PROOF_BY_POSITION_ROUTE: &str =
        "/v1/availability/transaction/{height}/{index}/proof";
    pub const TRANSACTION_PROOF_BY_HASH_ROUTE: &str =
        "/v1/availability/transaction/hash/{hash}/proof";
    pub const TRANSACTION_BY_POSITION_ROUTE: &str = "/v1/availability/transaction/{height}/{index}";
    pub const TRANSACTION_BY_HASH_ROUTE: &str = "/v1/availability/transaction/hash/{hash}";

    pub const BLOCK_SUMMARY_BY_HEIGHT_ROUTE: &str = "/v1/availability/block/summary/{height}";
    pub const BLOCK_SUMMARY_RANGE_ROUTE: &str = "/v1/availability/block/summaries/{from}/{until}";

    pub const LIMITS_ROUTE: &str = "/v1/availability/limits";
    pub const CERT2_BY_HEIGHT_ROUTE: &str = "/v1/availability/cert2/{height}";

    pub const STREAM_LEAVES_ROUTE: &str = "/v1/availability/stream/leaves/{height}";
    pub const STREAM_HEADERS_ROUTE: &str = "/v1/availability/stream/headers/{height}";
    pub const STREAM_BLOCKS_ROUTE: &str = "/v1/availability/stream/blocks/{height}";
    pub const STREAM_PAYLOADS_ROUTE: &str = "/v1/availability/stream/payloads/{height}";
    pub const STREAM_VID_COMMON_ROUTE: &str = "/v1/availability/stream/vid/common/{height}";
    pub const STREAM_TRANSACTIONS_ROUTE: &str = "/v1/availability/stream/transactions/{height}";
    pub const STREAM_TRANSACTIONS_NS_ROUTE: &str =
        "/v1/availability/stream/transactions/{height}/namespace/{namespace}";

    pub const STREAM_NAMESPACE_PROOFS_ROUTE: &str =
        "/v1/availability/stream/blocks/{height}/namespace/{namespace}";

    pub const BLOCK_STATE_PATH_BY_HEIGHT_ROUTE: &str = "/v1/block-state/{height}/{key}";
    pub const BLOCK_STATE_PATH_BY_COMMIT_ROUTE: &str = "/v1/block-state/commit/{commit}/{key}";
    pub const BLOCK_STATE_HEIGHT_ROUTE: &str = "/v1/block-state/block-height";

    pub const FEE_STATE_PATH_BY_HEIGHT_ROUTE: &str = "/v1/fee-state/{height}/{key}";
    pub const FEE_STATE_PATH_BY_COMMIT_ROUTE: &str = "/v1/fee-state/commit/{commit}/{key}";
    pub const FEE_STATE_HEIGHT_ROUTE: &str = "/v1/fee-state/block-height";
    pub const FEE_STATE_BALANCE_LATEST_ROUTE: &str = "/v1/fee-state/fee-balance/latest/{address}";

    pub const STATUS_BLOCK_HEIGHT_ROUTE: &str = "/v1/status/block-height";
    pub const STATUS_SUCCESS_RATE_ROUTE: &str = "/v1/status/success-rate";
    pub const STATUS_TIME_SINCE_LAST_DECIDE_ROUTE: &str = "/v1/status/time-since-last-decide";
    pub const STATUS_METRICS_ROUTE: &str = "/v1/status/metrics";

    pub const CONFIG_HOTSHOT_ROUTE: &str = "/v1/config/hotshot";
    pub const CONFIG_ENV_ROUTE: &str = "/v1/config/env";
    pub const CONFIG_RUNTIME_ROUTE: &str = "/v1/config/runtime";

    pub const NODE_BLOCK_HEIGHT_ROUTE: &str = "/v1/node/block-height";

    pub const NODE_TRANSACTIONS_COUNT_ROUTE: &str = "/v1/node/transactions/count";
    pub const NODE_TRANSACTIONS_COUNT_TO_ROUTE: &str = "/v1/node/transactions/count/{to}";
    pub const NODE_TRANSACTIONS_COUNT_FROM_TO_ROUTE: &str =
        "/v1/node/transactions/count/{from}/{to}";
    pub const NODE_TRANSACTIONS_COUNT_NS_ROUTE: &str =
        "/v1/node/transactions/count/namespace/{namespace}";
    pub const NODE_TRANSACTIONS_COUNT_NS_TO_ROUTE: &str =
        "/v1/node/transactions/count/namespace/{namespace}/{to}";
    pub const NODE_TRANSACTIONS_COUNT_NS_FROM_TO_ROUTE: &str =
        "/v1/node/transactions/count/namespace/{namespace}/{from}/{to}";

    pub const NODE_PAYLOADS_SIZE_ROUTE: &str = "/v1/node/payloads/size";
    pub const NODE_PAYLOADS_SIZE_TO_ROUTE: &str = "/v1/node/payloads/size/{to}";
    pub const NODE_PAYLOADS_SIZE_FROM_TO_ROUTE: &str = "/v1/node/payloads/size/{from}/{to}";
    pub const NODE_PAYLOADS_TOTAL_SIZE_ROUTE: &str = "/v1/node/payloads/total-size";
    pub const NODE_PAYLOADS_SIZE_NS_ROUTE: &str = "/v1/node/payloads/size/namespace/{namespace}";
    pub const NODE_PAYLOADS_SIZE_NS_TO_ROUTE: &str =
        "/v1/node/payloads/size/namespace/{namespace}/{to}";
    pub const NODE_PAYLOADS_SIZE_NS_FROM_TO_ROUTE: &str =
        "/v1/node/payloads/size/namespace/{namespace}/{from}/{to}";

    pub const NODE_VID_SHARE_BY_HEIGHT_ROUTE: &str = "/v1/node/vid/share/{height}";
    pub const NODE_VID_SHARE_BY_HASH_ROUTE: &str = "/v1/node/vid/share/hash/{hash}";
    pub const NODE_VID_SHARE_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/node/vid/share/payload-hash/{payload_hash}";

    pub const NODE_SYNC_STATUS_ROUTE: &str = "/v1/node/sync-status";

    pub const NODE_HEADER_WINDOW_TIME_ROUTE: &str = "/v1/node/header/window/{start}/{end}";
    pub const NODE_HEADER_WINDOW_HEIGHT_ROUTE: &str = "/v1/node/header/window/from/{height}/{end}";
    pub const NODE_HEADER_WINDOW_HASH_ROUTE: &str = "/v1/node/header/window/from/hash/{hash}/{end}";

    pub const NODE_LIMITS_ROUTE: &str = "/v1/node/limits";

    pub const NODE_STAKE_TABLE_CURRENT_ROUTE: &str = "/v1/node/stake-table/current";
    pub const NODE_STAKE_TABLE_ROUTE: &str = "/v1/node/stake-table/{epoch_number}";
    pub const NODE_DA_STAKE_TABLE_CURRENT_ROUTE: &str = "/v1/node/da-stake-table/current";
    pub const NODE_DA_STAKE_TABLE_ROUTE: &str = "/v1/node/da-stake-table/{epoch_number}";

    pub const NODE_VALIDATORS_ROUTE: &str = "/v1/node/validators/{epoch_number}";
    pub const NODE_ALL_VALIDATORS_ROUTE: &str =
        "/v1/node/all-validators/{epoch_number}/{offset}/{limit}";

    pub const NODE_PROPOSAL_PARTICIPATION_CURRENT_ROUTE: &str =
        "/v1/node/participation/proposal/current";
    pub const NODE_PROPOSAL_PARTICIPATION_ROUTE: &str = "/v1/node/participation/proposal/{epoch}";
    pub const NODE_VOTE_PARTICIPATION_CURRENT_ROUTE: &str = "/v1/node/participation/vote/current";
    pub const NODE_VOTE_PARTICIPATION_ROUTE: &str = "/v1/node/participation/vote/{epoch}";

    pub const NODE_BLOCK_REWARD_ROUTE: &str = "/v1/node/block-reward";
    pub const NODE_BLOCK_REWARD_EPOCH_ROUTE: &str = "/v1/node/block-reward/epoch/{epoch_number}";

    pub const NODE_OLDEST_BLOCK_ROUTE: &str = "/v1/node/oldest-block";
    pub const NODE_OLDEST_LEAF_ROUTE: &str = "/v1/node/oldest-leaf";

    // Catchup routes (under /v1/catchup)
    pub const CATCHUP_ACCOUNT_ROUTE: &str = "/v1/catchup/{height}/{view}/account/{address}";
    pub const CATCHUP_ACCOUNTS_ROUTE: &str = "/v1/catchup/{height}/{view}/accounts";
    pub const CATCHUP_BLOCKS_ROUTE: &str = "/v1/catchup/{height}/{view}/blocks";
    pub const CATCHUP_CHAINCONFIG_ROUTE: &str = "/v1/catchup/chain-config/{commitment}";
    pub const CATCHUP_LEAFCHAIN_ROUTE: &str = "/v1/catchup/{height}/leafchain";
    pub const CATCHUP_CERT2_ROUTE: &str = "/v1/catchup/{height}/cert2";
    pub const CATCHUP_REWARD_ACCOUNT_ROUTE: &str =
        "/v1/catchup/{height}/{view}/reward-account/{address}";
    pub const CATCHUP_REWARD_ACCOUNTS_ROUTE: &str = "/v1/catchup/{height}/{view}/reward-accounts";
    pub const CATCHUP_REWARD_ACCOUNT_V2_ROUTE: &str =
        "/v1/catchup/{height}/{view}/reward-account-v2/{address}";
    pub const CATCHUP_REWARD_ACCOUNTS_V2_ROUTE: &str =
        "/v1/catchup/{height}/{view}/reward-accounts-v2";
    pub const CATCHUP_REWARD_AMOUNTS_ROUTE: &str =
        "/v1/catchup/{height}/reward-amounts/{limit}/{offset}";
    pub const CATCHUP_REWARD_MERKLE_TREE_V2_ROUTE: &str =
        "/v1/catchup/reward-merkle-tree-v2/{height}/{view}";
    pub const CATCHUP_STATE_CERT_ROUTE: &str = "/v1/catchup/{epoch}/state-cert";

    // Submit
    pub const SUBMIT_ROUTE: &str = "/v1/submit/submit";

    // State signature
    pub const STATE_SIGNATURE_BLOCK_ROUTE: &str = "/v1/state-signature/block/{height}";

    // HotShot events
    pub const HOTSHOT_EVENTS_STREAM_ROUTE: &str = "/v1/hotshot-events/events";
    pub const HOTSHOT_EVENTS_STARTUP_ROUTE: &str = "/v1/hotshot-events/startup_info";

    // Light client
    pub const LC_LEAF_BY_HEIGHT_ROUTE: &str = "/v1/light-client/leaf/{height}";
    pub const LC_LEAF_BY_HEIGHT_FINALIZED_ROUTE: &str =
        "/v1/light-client/leaf/{height}/{finalized}";
    pub const LC_LEAF_BY_HASH_ROUTE: &str = "/v1/light-client/leaf/hash/{hash}";
    pub const LC_LEAF_BY_HASH_FINALIZED_ROUTE: &str =
        "/v1/light-client/leaf/hash/{hash}/{finalized}";
    pub const LC_LEAF_BY_BLOCK_HASH_ROUTE: &str = "/v1/light-client/leaf/block-hash/{block_hash}";
    pub const LC_LEAF_BY_BLOCK_HASH_FINALIZED_ROUTE: &str =
        "/v1/light-client/leaf/block-hash/{block_hash}/{finalized}";
    pub const LC_LEAF_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/light-client/leaf/payload-hash/{payload_hash}";
    pub const LC_LEAF_BY_PAYLOAD_HASH_FINALIZED_ROUTE: &str =
        "/v1/light-client/leaf/payload-hash/{payload_hash}/{finalized}";

    pub const LC_HEADER_BY_HEIGHT_ROUTE: &str = "/v1/light-client/header/{root}/{height}";
    pub const LC_HEADER_BY_HASH_ROUTE: &str = "/v1/light-client/header/{root}/hash/{hash}";
    pub const LC_HEADER_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/light-client/header/{root}/payload-hash/{payload_hash}";

    pub const LC_STAKE_TABLE_ROUTE: &str = "/v1/light-client/stake-table/{epoch}";

    pub const LC_PAYLOAD_ROUTE: &str = "/v1/light-client/payload/{height}";
    pub const LC_PAYLOAD_RANGE_ROUTE: &str = "/v1/light-client/payload/{start}/{end}";

    pub const LC_NAMESPACE_ROUTE: &str = "/v1/light-client/namespace/{height}/{namespace}";
    pub const LC_NAMESPACE_RANGE_ROUTE: &str =
        "/v1/light-client/namespace/{start}/{end}/{namespace}";
    pub const LC_NAMESPACES_RANGE_ROUTE: &str =
        "/v1/light-client/namespaces/{start}/{end}/{namespaces}";

    // Explorer
    pub const EXPLORER_BLOCK_DETAIL_BY_HEIGHT_ROUTE: &str = "/v1/explorer/block/{height}";
    pub const EXPLORER_BLOCK_DETAIL_BY_HASH_ROUTE: &str = "/v1/explorer/block/hash/{hash}";
    pub const EXPLORER_BLOCK_SUMMARIES_LATEST_ROUTE: &str = "/v1/explorer/blocks/latest/{limit}";
    pub const EXPLORER_BLOCK_SUMMARIES_FROM_ROUTE: &str = "/v1/explorer/blocks/{from}/{limit}";
    pub const EXPLORER_TX_DETAIL_BY_POSITION_ROUTE: &str =
        "/v1/explorer/transaction/{height}/{offset}";
    pub const EXPLORER_TX_DETAIL_BY_HASH_ROUTE: &str = "/v1/explorer/transaction/hash/{hash}";

    pub const EXPLORER_TX_SUMMARIES_LATEST_ROUTE: &str = "/v1/explorer/transactions/latest/{limit}";
    pub const EXPLORER_TX_SUMMARIES_FROM_ROUTE: &str =
        "/v1/explorer/transactions/from/{height}/{offset}/{limit}";
    pub const EXPLORER_TX_SUMMARIES_BY_HASH_ROUTE: &str =
        "/v1/explorer/transactions/hash/{hash}/{limit}";

    pub const EXPLORER_TX_SUMMARIES_LATEST_BLOCK_ROUTE: &str =
        "/v1/explorer/transactions/latest/{limit}/block/{block}";
    pub const EXPLORER_TX_SUMMARIES_FROM_BLOCK_ROUTE: &str =
        "/v1/explorer/transactions/from/{height}/{offset}/{limit}/block/{block}";
    pub const EXPLORER_TX_SUMMARIES_BY_HASH_BLOCK_ROUTE: &str =
        "/v1/explorer/transactions/hash/{hash}/{limit}/block/{block}";

    pub const EXPLORER_TX_SUMMARIES_LATEST_NS_ROUTE: &str =
        "/v1/explorer/transactions/latest/{limit}/namespace/{namespace}";
    pub const EXPLORER_TX_SUMMARIES_FROM_NS_ROUTE: &str =
        "/v1/explorer/transactions/from/{height}/{offset}/{limit}/namespace/{namespace}";
    pub const EXPLORER_TX_SUMMARIES_BY_HASH_NS_ROUTE: &str =
        "/v1/explorer/transactions/hash/{hash}/{limit}/namespace/{namespace}";

    pub const EXPLORER_SUMMARY_ROUTE: &str = "/v1/explorer/explorer-summary";
    pub const EXPLORER_SEARCH_ROUTE: &str = "/v1/explorer/search/{query}";

    // Token
    pub const TOKEN_TOTAL_MINTED_SUPPLY_ROUTE: &str = "/v1/token/total-minted-supply";
    pub const TOKEN_CIRCULATING_SUPPLY_ROUTE: &str = "/v1/token/circulating-supply";
    pub const TOKEN_CIRCULATING_SUPPLY_ETHEREUM_ROUTE: &str =
        "/v1/token/circulating-supply-ethereum";
    pub const TOKEN_TOTAL_ISSUED_SUPPLY_ROUTE: &str = "/v1/token/total-issued-supply";
    pub const TOKEN_TOTAL_REWARD_DISTRIBUTED_ROUTE: &str = "/v1/token/total-reward-distributed";

    // Database (diagnostic)
    pub const DATABASE_TABLE_SIZES_ROUTE: &str = "/v1/database/table-sizes";

    // ---------------------------------------------------------------------
    // Path builders
    //
    // For each constant above, generate a function that returns the path
    // with `{placeholder}` segments substituted. Use these instead of
    // hand-formatted URL strings so that the route definition and the
    // request site stay in sync.
    // ---------------------------------------------------------------------

    // Reward state v2
    path_fn!(reward_claim_input, REWARD_CLAIM_INPUT_ROUTE, "height" => height, "address" => address);
    path_fn!(reward_balance, REWARD_BALANCE_ROUTE, "height" => height, "address" => address);
    path_fn!(latest_reward_balance, LATEST_REWARD_BALANCE_ROUTE, "address" => address);
    path_fn!(reward_account_proof, REWARD_ACCOUNT_PROOF_ROUTE, "height" => height, "address" => address);
    path_fn!(latest_reward_account_proof, LATEST_REWARD_ACCOUNT_PROOF_ROUTE, "address" => address);
    path_fn!(reward_amounts, REWARD_AMOUNTS_ROUTE, "height" => height, "offset" => offset, "limit" => limit);
    path_fn!(reward_merkle_tree_v2, REWARD_MERKLE_TREE_V2_ROUTE, "height" => height);

    // Availability — namespace proofs
    path_fn!(namespace_proof_by_height, NAMESPACE_PROOF_BY_HEIGHT_ROUTE, "height" => height, "namespace" => namespace);
    path_fn!(namespace_proof_by_hash, NAMESPACE_PROOF_BY_HASH_ROUTE, "hash" => hash, "namespace" => namespace);
    path_fn!(namespace_proof_by_payload_hash, NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE, "payload-hash" => payload_hash, "namespace" => namespace);
    path_fn!(namespace_proof_range, NAMESPACE_PROOF_RANGE_ROUTE, "from" => from, "until" => until, "namespace" => namespace);
    path_fn!(incorrect_encoding_proof, INCORRECT_ENCODING_PROOF_ROUTE, "block_number" => block_number, "namespace" => namespace);

    // Availability — state certificates
    path_fn!(state_cert_v1, STATE_CERT_V1_ROUTE, "epoch" => epoch);
    path_fn!(state_cert_v2, STATE_CERT_V2_ROUTE, "epoch" => epoch);

    // Availability — leaves
    path_fn!(leaf_by_height, LEAF_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(leaf_by_hash, LEAF_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(leaf_range, LEAF_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — headers
    path_fn!(header_by_height, HEADER_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(header_by_hash, HEADER_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(header_by_payload_hash, HEADER_BY_PAYLOAD_HASH_ROUTE, "payload_hash" => payload_hash);
    path_fn!(header_range, HEADER_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — blocks
    path_fn!(block_by_height, BLOCK_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(block_by_hash, BLOCK_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(block_by_payload_hash, BLOCK_BY_PAYLOAD_HASH_ROUTE, "payload_hash" => payload_hash);
    path_fn!(block_range, BLOCK_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — payloads
    path_fn!(payload_by_height, PAYLOAD_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(payload_by_hash, PAYLOAD_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(payload_by_block_hash, PAYLOAD_BY_BLOCK_HASH_ROUTE, "block_hash" => block_hash);
    path_fn!(payload_range, PAYLOAD_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — VID common
    path_fn!(vid_common_by_height, VID_COMMON_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(vid_common_by_hash, VID_COMMON_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(vid_common_by_payload_hash, VID_COMMON_BY_PAYLOAD_HASH_ROUTE, "payload_hash" => payload_hash);
    path_fn!(vid_common_range, VID_COMMON_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — transactions
    path_fn!(transaction_by_position_noproof, TRANSACTION_BY_POSITION_NOPROOF_ROUTE, "height" => height, "index" => index);
    path_fn!(transaction_by_hash_noproof, TRANSACTION_BY_HASH_NOPROOF_ROUTE, "hash" => hash);
    path_fn!(transaction_proof_by_position, TRANSACTION_PROOF_BY_POSITION_ROUTE, "height" => height, "index" => index);
    path_fn!(transaction_proof_by_hash, TRANSACTION_PROOF_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(transaction_by_position, TRANSACTION_BY_POSITION_ROUTE, "height" => height, "index" => index);
    path_fn!(transaction_by_hash, TRANSACTION_BY_HASH_ROUTE, "hash" => hash);

    // Availability — block summaries
    path_fn!(block_summary_by_height, BLOCK_SUMMARY_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(block_summary_range, BLOCK_SUMMARY_RANGE_ROUTE, "from" => from, "until" => until);

    // Availability — misc
    path_fn!(limits, LIMITS_ROUTE);
    path_fn!(cert2_by_height, CERT2_BY_HEIGHT_ROUTE, "height" => height);

    // Availability — streams
    path_fn!(stream_leaves, STREAM_LEAVES_ROUTE, "height" => height);
    path_fn!(stream_headers, STREAM_HEADERS_ROUTE, "height" => height);
    path_fn!(stream_blocks, STREAM_BLOCKS_ROUTE, "height" => height);
    path_fn!(stream_payloads, STREAM_PAYLOADS_ROUTE, "height" => height);
    path_fn!(stream_vid_common, STREAM_VID_COMMON_ROUTE, "height" => height);
    path_fn!(stream_transactions, STREAM_TRANSACTIONS_ROUTE, "height" => height);
    path_fn!(stream_transactions_ns, STREAM_TRANSACTIONS_NS_ROUTE, "height" => height, "namespace" => namespace);
    path_fn!(stream_namespace_proofs, STREAM_NAMESPACE_PROOFS_ROUTE, "height" => height, "namespace" => namespace);

    // Block state
    path_fn!(block_state_path_by_height, BLOCK_STATE_PATH_BY_HEIGHT_ROUTE, "height" => height, "key" => key);
    path_fn!(block_state_path_by_commit, BLOCK_STATE_PATH_BY_COMMIT_ROUTE, "commit" => commit, "key" => key);
    path_fn!(block_state_height, BLOCK_STATE_HEIGHT_ROUTE);

    // Fee state
    path_fn!(fee_state_path_by_height, FEE_STATE_PATH_BY_HEIGHT_ROUTE, "height" => height, "key" => key);
    path_fn!(fee_state_path_by_commit, FEE_STATE_PATH_BY_COMMIT_ROUTE, "commit" => commit, "key" => key);
    path_fn!(fee_state_height, FEE_STATE_HEIGHT_ROUTE);
    path_fn!(fee_state_balance_latest, FEE_STATE_BALANCE_LATEST_ROUTE, "address" => address);

    // Status
    path_fn!(status_block_height, STATUS_BLOCK_HEIGHT_ROUTE);
    path_fn!(status_success_rate, STATUS_SUCCESS_RATE_ROUTE);
    path_fn!(
        status_time_since_last_decide,
        STATUS_TIME_SINCE_LAST_DECIDE_ROUTE
    );
    path_fn!(status_metrics, STATUS_METRICS_ROUTE);

    // Config
    path_fn!(config_hotshot, CONFIG_HOTSHOT_ROUTE);
    path_fn!(config_env, CONFIG_ENV_ROUTE);
    path_fn!(config_runtime, CONFIG_RUNTIME_ROUTE);

    // Node — block height / counts / sizes
    path_fn!(node_block_height, NODE_BLOCK_HEIGHT_ROUTE);
    path_fn!(node_transactions_count, NODE_TRANSACTIONS_COUNT_ROUTE);
    path_fn!(node_transactions_count_to, NODE_TRANSACTIONS_COUNT_TO_ROUTE, "to" => to);
    path_fn!(node_transactions_count_from_to, NODE_TRANSACTIONS_COUNT_FROM_TO_ROUTE, "from" => from, "to" => to);
    path_fn!(node_transactions_count_ns, NODE_TRANSACTIONS_COUNT_NS_ROUTE, "namespace" => namespace);
    path_fn!(node_transactions_count_ns_to, NODE_TRANSACTIONS_COUNT_NS_TO_ROUTE, "namespace" => namespace, "to" => to);
    path_fn!(node_transactions_count_ns_from_to, NODE_TRANSACTIONS_COUNT_NS_FROM_TO_ROUTE, "namespace" => namespace, "from" => from, "to" => to);

    path_fn!(node_payloads_size, NODE_PAYLOADS_SIZE_ROUTE);
    path_fn!(node_payloads_size_to, NODE_PAYLOADS_SIZE_TO_ROUTE, "to" => to);
    path_fn!(node_payloads_size_from_to, NODE_PAYLOADS_SIZE_FROM_TO_ROUTE, "from" => from, "to" => to);
    path_fn!(node_payloads_total_size, NODE_PAYLOADS_TOTAL_SIZE_ROUTE);
    path_fn!(node_payloads_size_ns, NODE_PAYLOADS_SIZE_NS_ROUTE, "namespace" => namespace);
    path_fn!(node_payloads_size_ns_to, NODE_PAYLOADS_SIZE_NS_TO_ROUTE, "namespace" => namespace, "to" => to);
    path_fn!(node_payloads_size_ns_from_to, NODE_PAYLOADS_SIZE_NS_FROM_TO_ROUTE, "namespace" => namespace, "from" => from, "to" => to);

    // Node — VID shares
    path_fn!(node_vid_share_by_height, NODE_VID_SHARE_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(node_vid_share_by_hash, NODE_VID_SHARE_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(node_vid_share_by_payload_hash, NODE_VID_SHARE_BY_PAYLOAD_HASH_ROUTE, "payload_hash" => payload_hash);

    // Node — sync, header windows
    path_fn!(node_sync_status, NODE_SYNC_STATUS_ROUTE);
    path_fn!(node_header_window_time, NODE_HEADER_WINDOW_TIME_ROUTE, "start" => start, "end" => end);
    path_fn!(node_header_window_height, NODE_HEADER_WINDOW_HEIGHT_ROUTE, "height" => height, "end" => end);
    path_fn!(node_header_window_hash, NODE_HEADER_WINDOW_HASH_ROUTE, "hash" => hash, "end" => end);
    path_fn!(node_limits, NODE_LIMITS_ROUTE);

    // Node — stake table / validators / participation
    path_fn!(node_stake_table_current, NODE_STAKE_TABLE_CURRENT_ROUTE);
    path_fn!(node_stake_table, NODE_STAKE_TABLE_ROUTE, "epoch_number" => epoch_number);
    path_fn!(
        node_da_stake_table_current,
        NODE_DA_STAKE_TABLE_CURRENT_ROUTE
    );
    path_fn!(node_da_stake_table, NODE_DA_STAKE_TABLE_ROUTE, "epoch_number" => epoch_number);
    path_fn!(node_validators, NODE_VALIDATORS_ROUTE, "epoch_number" => epoch_number);
    path_fn!(node_all_validators, NODE_ALL_VALIDATORS_ROUTE, "epoch_number" => epoch_number, "offset" => offset, "limit" => limit);
    path_fn!(
        node_proposal_participation_current,
        NODE_PROPOSAL_PARTICIPATION_CURRENT_ROUTE
    );
    path_fn!(node_proposal_participation, NODE_PROPOSAL_PARTICIPATION_ROUTE, "epoch" => epoch);
    path_fn!(
        node_vote_participation_current,
        NODE_VOTE_PARTICIPATION_CURRENT_ROUTE
    );
    path_fn!(node_vote_participation, NODE_VOTE_PARTICIPATION_ROUTE, "epoch" => epoch);

    // Node — block reward / oldest
    path_fn!(node_block_reward, NODE_BLOCK_REWARD_ROUTE);
    path_fn!(node_block_reward_epoch, NODE_BLOCK_REWARD_EPOCH_ROUTE, "epoch_number" => epoch_number);
    path_fn!(node_oldest_block, NODE_OLDEST_BLOCK_ROUTE);
    path_fn!(node_oldest_leaf, NODE_OLDEST_LEAF_ROUTE);

    // Catchup
    path_fn!(catchup_account, CATCHUP_ACCOUNT_ROUTE, "height" => height, "view" => view, "address" => address);
    path_fn!(catchup_accounts, CATCHUP_ACCOUNTS_ROUTE, "height" => height, "view" => view);
    path_fn!(catchup_blocks, CATCHUP_BLOCKS_ROUTE, "height" => height, "view" => view);
    path_fn!(catchup_chainconfig, CATCHUP_CHAINCONFIG_ROUTE, "commitment" => commitment);
    path_fn!(catchup_leafchain, CATCHUP_LEAFCHAIN_ROUTE, "height" => height);
    path_fn!(catchup_cert2, CATCHUP_CERT2_ROUTE, "height" => height);
    path_fn!(catchup_reward_account, CATCHUP_REWARD_ACCOUNT_ROUTE, "height" => height, "view" => view, "address" => address);
    path_fn!(catchup_reward_accounts, CATCHUP_REWARD_ACCOUNTS_ROUTE, "height" => height, "view" => view);
    path_fn!(catchup_reward_account_v2, CATCHUP_REWARD_ACCOUNT_V2_ROUTE, "height" => height, "view" => view, "address" => address);
    path_fn!(catchup_reward_accounts_v2, CATCHUP_REWARD_ACCOUNTS_V2_ROUTE, "height" => height, "view" => view);
    path_fn!(catchup_reward_amounts, CATCHUP_REWARD_AMOUNTS_ROUTE, "height" => height, "limit" => limit, "offset" => offset);
    path_fn!(catchup_reward_merkle_tree_v2, CATCHUP_REWARD_MERKLE_TREE_V2_ROUTE, "height" => height, "view" => view);
    path_fn!(catchup_state_cert, CATCHUP_STATE_CERT_ROUTE, "epoch" => epoch);

    // Submit
    path_fn!(submit, SUBMIT_ROUTE);

    // State signature
    path_fn!(state_signature_block, STATE_SIGNATURE_BLOCK_ROUTE, "height" => height);

    // HotShot events
    path_fn!(hotshot_events_stream, HOTSHOT_EVENTS_STREAM_ROUTE);
    path_fn!(hotshot_events_startup, HOTSHOT_EVENTS_STARTUP_ROUTE);

    // Light client
    path_fn!(lc_leaf_by_height, LC_LEAF_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(lc_leaf_by_height_finalized, LC_LEAF_BY_HEIGHT_FINALIZED_ROUTE, "height" => height, "finalized" => finalized);
    path_fn!(lc_leaf_by_hash, LC_LEAF_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(lc_leaf_by_hash_finalized, LC_LEAF_BY_HASH_FINALIZED_ROUTE, "hash" => hash, "finalized" => finalized);
    path_fn!(lc_leaf_by_block_hash, LC_LEAF_BY_BLOCK_HASH_ROUTE, "block_hash" => block_hash);
    path_fn!(lc_leaf_by_block_hash_finalized, LC_LEAF_BY_BLOCK_HASH_FINALIZED_ROUTE, "block_hash" => block_hash, "finalized" => finalized);
    path_fn!(lc_leaf_by_payload_hash, LC_LEAF_BY_PAYLOAD_HASH_ROUTE, "payload_hash" => payload_hash);
    path_fn!(lc_leaf_by_payload_hash_finalized, LC_LEAF_BY_PAYLOAD_HASH_FINALIZED_ROUTE, "payload_hash" => payload_hash, "finalized" => finalized);
    path_fn!(lc_header_by_height, LC_HEADER_BY_HEIGHT_ROUTE, "root" => root, "height" => height);
    path_fn!(lc_header_by_hash, LC_HEADER_BY_HASH_ROUTE, "root" => root, "hash" => hash);
    path_fn!(lc_header_by_payload_hash, LC_HEADER_BY_PAYLOAD_HASH_ROUTE, "root" => root, "payload_hash" => payload_hash);
    path_fn!(lc_stake_table, LC_STAKE_TABLE_ROUTE, "epoch" => epoch);
    path_fn!(lc_payload, LC_PAYLOAD_ROUTE, "height" => height);
    path_fn!(lc_payload_range, LC_PAYLOAD_RANGE_ROUTE, "start" => start, "end" => end);
    path_fn!(lc_namespace, LC_NAMESPACE_ROUTE, "height" => height, "namespace" => namespace);
    path_fn!(lc_namespace_range, LC_NAMESPACE_RANGE_ROUTE, "start" => start, "end" => end, "namespace" => namespace);
    path_fn!(lc_namespaces_range, LC_NAMESPACES_RANGE_ROUTE, "start" => start, "end" => end, "namespaces" => namespaces);

    // Explorer — blocks
    path_fn!(explorer_block_detail_by_height, EXPLORER_BLOCK_DETAIL_BY_HEIGHT_ROUTE, "height" => height);
    path_fn!(explorer_block_detail_by_hash, EXPLORER_BLOCK_DETAIL_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(explorer_block_summaries_latest, EXPLORER_BLOCK_SUMMARIES_LATEST_ROUTE, "limit" => limit);
    path_fn!(explorer_block_summaries_from, EXPLORER_BLOCK_SUMMARIES_FROM_ROUTE, "from" => from, "limit" => limit);

    // Explorer — transactions
    path_fn!(explorer_tx_detail_by_position, EXPLORER_TX_DETAIL_BY_POSITION_ROUTE, "height" => height, "offset" => offset);
    path_fn!(explorer_tx_detail_by_hash, EXPLORER_TX_DETAIL_BY_HASH_ROUTE, "hash" => hash);
    path_fn!(explorer_tx_summaries_latest, EXPLORER_TX_SUMMARIES_LATEST_ROUTE, "limit" => limit);
    path_fn!(explorer_tx_summaries_from, EXPLORER_TX_SUMMARIES_FROM_ROUTE, "height" => height, "offset" => offset, "limit" => limit);
    path_fn!(explorer_tx_summaries_by_hash, EXPLORER_TX_SUMMARIES_BY_HASH_ROUTE, "hash" => hash, "limit" => limit);
    path_fn!(explorer_tx_summaries_latest_block, EXPLORER_TX_SUMMARIES_LATEST_BLOCK_ROUTE, "limit" => limit, "block" => block);
    path_fn!(explorer_tx_summaries_from_block, EXPLORER_TX_SUMMARIES_FROM_BLOCK_ROUTE, "height" => height, "offset" => offset, "limit" => limit, "block" => block);
    path_fn!(explorer_tx_summaries_by_hash_block, EXPLORER_TX_SUMMARIES_BY_HASH_BLOCK_ROUTE, "hash" => hash, "limit" => limit, "block" => block);
    path_fn!(explorer_tx_summaries_latest_ns, EXPLORER_TX_SUMMARIES_LATEST_NS_ROUTE, "limit" => limit, "namespace" => namespace);
    path_fn!(explorer_tx_summaries_from_ns, EXPLORER_TX_SUMMARIES_FROM_NS_ROUTE, "height" => height, "offset" => offset, "limit" => limit, "namespace" => namespace);
    path_fn!(explorer_tx_summaries_by_hash_ns, EXPLORER_TX_SUMMARIES_BY_HASH_NS_ROUTE, "hash" => hash, "limit" => limit, "namespace" => namespace);
    path_fn!(explorer_summary, EXPLORER_SUMMARY_ROUTE);
    path_fn!(explorer_search, EXPLORER_SEARCH_ROUTE, "query" => query);

    // Token
    path_fn!(token_total_minted_supply, TOKEN_TOTAL_MINTED_SUPPLY_ROUTE);
    path_fn!(token_circulating_supply, TOKEN_CIRCULATING_SUPPLY_ROUTE);
    path_fn!(
        token_circulating_supply_ethereum,
        TOKEN_CIRCULATING_SUPPLY_ETHEREUM_ROUTE
    );
    path_fn!(token_total_issued_supply, TOKEN_TOTAL_ISSUED_SUPPLY_ROUTE);
    path_fn!(
        token_total_reward_distributed,
        TOKEN_TOTAL_REWARD_DISTRIBUTED_ROUTE
    );

    // Database (diagnostic)
    path_fn!(database_table_sizes, DATABASE_TABLE_SIZES_ROUTE);
}

pub mod v2 {
    use super::Route;

    pub const REWARD_CLAIM_INPUT_ROUTE: Route = Route {
        http: "/v2/rewards/claim-input",
        grpc: "/espresso.api.v2.RewardService/GetRewardClaimInput",
        description: "Get reward claim input for L1 contract submission. Returns lifetime rewards \
                      and Merkle proof needed to call claimRewards() on the L1 contract.",
        tag: "Rewards",
    };

    pub const REWARD_BALANCE_ROUTE: Route = Route {
        http: "/v2/rewards/balance",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalance",
        description: "Get reward balance for an address at the latest finalized height",
        tag: "Rewards",
    };

    pub const REWARD_ACCOUNT_PROOF_ROUTE: Route = Route {
        http: "/v2/rewards/proof",
        grpc: "/espresso.api.v2.RewardService/GetRewardAccountProof",
        description: "Get Merkle proof for a reward account at the latest finalized height. \
                      Returns V2 proof with Keccak256 hashing",
        tag: "Rewards",
    };

    pub const REWARD_BALANCES_ROUTE: Route = Route {
        http: "/v2/rewards/balances",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalances",
        description: "Get paginated list of all reward balances at a specific height. Limit must \
                      be ≤ 10000",
        tag: "Rewards",
    };

    pub const REWARD_MERKLE_TREE_V2_ROUTE: Route = Route {
        http: "/v2/rewards/tree",
        grpc: "/espresso.api.v2.RewardService/GetRewardMerkleTreeV2",
        description: "Get raw RewardMerkleTreeV2 snapshot at a given height. Returns serialized \
                      merkle tree data",
        tag: "Rewards",
    };

    pub const OPENAPI_SPEC_ROUTE: &str = "/v2/docs/openapi.json";
    pub const SWAGGER_ROUTE: &str = "/v2";
    pub const SCALAR_ROUTE: &str = "/v2/scalar";
    pub const REDOC_ROUTE: &str = "/v2/redoc";

    pub const NAMESPACE_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/namespace-proof",
        grpc: "/espresso.api.v2.DataService/GetNamespaceProof",
        description: "Get namespace proof(s) for the specified namespace. Use '?block={height}' \
                      for a single block, or '?from={start}&to={end}' for a range. Returns \
                      transactions for the namespace along with cryptographic proof(s) of \
                      completeness.",
        tag: "Data",
    };

    pub const INCORRECT_ENCODING_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/incorrect-encoding-proof",
        grpc: "/espresso.api.v2.DataService/GetIncorrectEncodingProof",
        description: "Generate a fraud proof showing incorrect namespace encoding for a specific \
                      block. Query param 'block' specifies the block height. Used to challenge \
                      invalid block proposals.",
        tag: "Data",
    };

    pub const STATE_CERTIFICATE_ROUTE: Route = Route {
        http: "/v2/consensus/state-certificate",
        grpc: "/espresso.api.v2.ConsensusService/GetStateCertificate",
        description: "Get light client state update certificate for an epoch. Used to update L1 \
                      contracts with new stake table information.",
        tag: "Consensus",
    };

    pub const STAKE_TABLE_ROUTE: Route = Route {
        http: "/v2/consensus/stake-table",
        grpc: "/espresso.api.v2.ConsensusService/GetStakeTable",
        description: "Get stake table for an epoch.",
        tag: "Consensus",
    };
}
