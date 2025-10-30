use vbs::version::Version;

// Re-export types which haven't changed since the last minor version.
pub use super::v0_4::{
    ADVZNsProof, ADVZTxProof, AccountQueryData, AvidMIncorrectEncodingNsProof, AvidMNsProof,
    AvidMTxProof, BlockMerkleCommitment, BlockMerkleTree, BlockSize, BuilderSignature, ChainConfig,
    ChainId, Delta, FeeAccount, FeeAccountProof, FeeAmount, FeeInfo, FeeMerkleCommitment,
    FeeMerkleProof, FeeMerkleTree, Header, Index, Iter, L1BlockInfo, L1Client, L1ClientOptions,
    L1Snapshot, NamespaceId, NsIndex, NsIter, NsPayload, NsPayloadBuilder, NsPayloadByteLen,
    NsPayloadOwned, NsPayloadRange, NsTable, NsTableBuilder, NsTableValidationError, NumNss,
    NumTxs, NumTxsRange, NumTxsUnchecked, Payload, PayloadByteLen, ResolvableChainConfig,
    RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardClaimError,
    RewardMerkleCommitmentV2, RewardMerkleProofV2, RewardMerkleTreeV2, TimeBasedUpgrade,
    Transaction, TxIndex, TxIter, TxPayload, TxPayloadRange, TxTableEntries, TxTableEntriesRange,
    Upgrade, UpgradeMode, UpgradeType, ViewBasedUpgrade, BLOCK_MERKLE_TREE_HEIGHT,
    FEE_MERKLE_TREE_HEIGHT, NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN, NUM_TXS_BYTE_LEN,
    REWARD_MERKLE_TREE_V2_ARITY, REWARD_MERKLE_TREE_V2_HEIGHT, TX_OFFSET_BYTE_LEN,
};

pub const VERSION: Version = Version { major: 0, minor: 5 };
