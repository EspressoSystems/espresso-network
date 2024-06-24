use vbs::version::Version;

// Re-export types which haven't changed since the last minor vresion.
pub use super::v0_1::{
    AccountQueryData, BlockMerkleCommitment, BlockMerkleTree, BuilderSignature, ChainConfig,
    ChainId, FeeAccount, FeeAccountProof, FeeAmount, FeeInfo, FeeMerkleCommitment, FeeMerkleTree,
    L1BlockInfo, L1Client, L1Snapshot, NameSpaceTable, NamespaceId, NamespaceInfo, NamespaceProof,
    NodeState, NsTable, Payload, ResolvableChainConfig, StateCatchup, Table, TableWordTraits,
    Transaction, TxIndex, TxIterator, TxTable, TxTableEntry, TxTableEntryWord, ValidatedState,
};

pub const VERSION: Version = Version { major: 0, minor: 1 };

mod header;

pub use header::Header;
