use vbs::version::Version;

// Re-export types which haven't changed since the last minor version.
pub use super::v0_1::{
    AccountQueryData, BlockMerkleCommitment, BlockMerkleTree, BlockSize, BuilderSignature,
    ChainConfig, ChainId, Delta, FeeAccount, FeeAccountProof, FeeAmount, FeeInfo,
    FeeMerkleCommitment, FeeMerkleProof, FeeMerkleTree, Header, Index, Iter, L1BlockInfo, L1Client,
    L1ClientOptions, L1Snapshot, NamespaceId, NsIndex, NsIter, NsPayload, NsPayloadBuilder,
    NsPayloadByteLen, NsPayloadOwned, NsPayloadRange, ADVZNsProof, NsTable, NsTableBuilder,
    NsTableValidationError, NumNss, NumTxs, NumTxsRange, NumTxsUnchecked, Payload, PayloadByteLen,
    ResolvableChainConfig, TimeBasedUpgrade, Transaction, TxIndex, TxIter, TxPayload,
    TxPayloadRange, TxProof, TxTableEntries, TxTableEntriesRange, Upgrade, UpgradeMode,
    UpgradeType, ViewBasedUpgrade, BLOCK_MERKLE_TREE_HEIGHT, FEE_MERKLE_TREE_HEIGHT,
    NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN, NUM_TXS_BYTE_LEN, TX_OFFSET_BYTE_LEN, };

pub const VERSION: Version = Version { major: 0, minor: 2 };
