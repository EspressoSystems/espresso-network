use vbs::version::Version;

// Re-export types which haven't changed since the last minor version.
pub use super::v0_1::{
    ADVZNsProof, ADVZTxProof, AccountQueryData, BLOCK_MERKLE_TREE_HEIGHT, BlockMerkleCommitment,
    BlockMerkleTree, BlockSize, BuilderSignature, ChainConfig, ChainId, FEE_MERKLE_TREE_HEIGHT,
    FeeAccount, FeeAccountProof, FeeAmount, FeeInfo, FeeMerkleCommitment, FeeMerkleProof,
    FeeMerkleTree, Header, Index, Iter, L1BlockInfo, L1Client, L1ClientOptions, L1Snapshot,
    NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN, NUM_TXS_BYTE_LEN, NamespaceId, NsIndex,
    NsIter, NsPayload, NsPayloadBuilder, NsPayloadByteLen, NsPayloadOwned, NsPayloadRange, NsTable,
    NsTableBuilder, NsTableValidationError, NumNss, NumTxs, NumTxsRange, NumTxsUnchecked, Payload,
    PayloadByteLen, ResolvableChainConfig, TX_OFFSET_BYTE_LEN, TimeBasedUpgrade, Transaction,
    TxIndex, TxIter, TxPayload, TxPayloadRange, TxTableEntries, TxTableEntriesRange, Upgrade,
    UpgradeMode, UpgradeType, ViewBasedUpgrade,
};

pub const VERSION: Version = Version { major: 0, minor: 2 };
