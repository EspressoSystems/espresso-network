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
