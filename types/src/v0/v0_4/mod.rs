use vbs::version::Version;

// Re-export types which haven't changed since the last minor version.
pub use super::v0_1::{
    ADVZNsProof, ADVZTxProof, AccountQueryData, BlockMerkleCommitment, BlockMerkleTree, BlockSize,
    BuilderSignature, ChainId, FeeAccount, FeeAccountProof, FeeAmount, FeeInfo,
    FeeMerkleCommitment, FeeMerkleProof, FeeMerkleTree, Index, Iter, L1BlockInfo, L1Client,
    L1ClientOptions, L1Snapshot, NamespaceId, NsIndex, NsIter, NsPayload, NsPayloadBuilder,
    NsPayloadByteLen, NsPayloadOwned, NsPayloadRange, NsTable, NsTableBuilder,
    NsTableValidationError, NumNss, NumTxs, NumTxsRange, NumTxsUnchecked, Payload, PayloadByteLen,
    TimeBasedUpgrade, Transaction, TxIndex, TxIter, TxPayload, TxPayloadRange, TxTableEntries,
    TxTableEntriesRange, Upgrade, UpgradeMode, UpgradeType, ViewBasedUpgrade,
    BLOCK_MERKLE_TREE_HEIGHT, FEE_MERKLE_TREE_HEIGHT, NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN,
    NUM_NSS_BYTE_LEN, NUM_TXS_BYTE_LEN, TX_OFFSET_BYTE_LEN,
};

pub use super::v0_3::{
    AvidMIncorrectEncodingNsProof, AvidMNsProof, AvidMTxProof, ChainConfig, ResolvableChainConfig,
};

pub const VERSION: Version = Version { major: 0, minor: 4 };

mod header;
mod state;
mod state_cert;

pub use header::*;
pub use state::*;
pub use state_cert::*;
