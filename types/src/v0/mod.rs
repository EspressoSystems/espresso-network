use hotshot::traits::election::static_committee::GeneralStaticCommittee;
use hotshot_types::{
    data::ViewNumber,
    signature_key::BLSPubKey,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use serde::{Deserialize, Serialize};
use snafu::Snafu;

mod header;
mod impls;

pub use header::Header;
pub use impls::validate_proposal;

// This is the single source of truth for minor versions supported by this major version.
//
// It is written as a higher-level macro which takes a macro invocation as an argument and appends
// the comma-separated list of minor version identifiers to the arguments of the given invocation.
// This is to get around Rust's lazy macro expansion: this macro forces expansion of the given
// invocation. We would rather write something like `some_macro!(args, minor_versions!())`, but the
// `minor_versions!()` argument would not be expanded for pattern-matching in `some_macro!`, so
// instead we write `with_minor_versions!(some_macro!(args))`.
macro_rules! with_minor_versions {
    ($m:ident!($($arg:tt),*)) => {
        $m!($($arg,)* v0_1, v0_2, v0_3);
    };
}

// Define sub-modules for each supported minor version.
macro_rules! define_modules {
    ($($m:ident),+) => {
        $(pub mod $m;)+
    };
}
with_minor_versions!(define_modules!());

macro_rules! assert_eq_all_versions_of_type {
    ($t:ident, $($m:ident),+) => {
        static_assertions::assert_type_eq_all!($($m::$t),+);
    };
}

macro_rules! reexport_latest_version_of_type {
    ($t:ident, $m:ident) => { pub use $m::$t; };
    ($t:ident, $m1:ident, $($m:ident),+) => {
        reexport_latest_version_of_type!($t, $($m),+);
    }
}

/// Re-export types which have not changed across any minor version.
macro_rules! reexport_unchanged_types {
    ($($t:ident),+ $(,)?) => {
        $(
            with_minor_versions!(assert_eq_all_versions_of_type!($t));
            with_minor_versions!(reexport_latest_version_of_type!($t));
        )+
    }
}
reexport_unchanged_types!(
    AccountQueryData,
    BlockMerkleCommitment,
    BlockMerkleTree,
    BuilderSignature,
    BuilderValidationError,
    ChainConfig,
    ChainId,
    Delta,
    FeeAccount,
    FeeAccountProof,
    FeeAmount,
    FeeError,
    FeeInfo,
    FeeMerkleCommitment,
    FeeMerkleProof,
    FeeMerkleTree,
    GenesisHeader,
    Index,
    Iter,
    L1BlockInfo,
    L1Client,
    L1Snapshot,
    NamespaceId,
    NodeState,
    NsIndex,
    NsIter,
    NsPayload,
    NsPayloadBuilder,
    NsPayloadByteLen,
    NsPayloadOwned,
    NsPayloadRange,
    NsProof,
    NsTable,
    NsTableBuilder,
    NsTableValidationError,
    NumNss,
    NumTxs,
    NumTxsRange,
    NumTxsUnchecked,
    Payload,
    PayloadByteLen,
    ProposalValidationError,
    ResolvableChainConfig,
    StateValidationError,
    Timestamp,
    Transaction,
    TxIndex,
    TxIter,
    TxPayload,
    TxPayloadRange,
    TxProof,
    TxTableEntries,
    TxTableEntriesRange,
    Upgrade,
    UpgradeType,
    ValidatedState,
    BlockSize,
);

#[derive(
    Clone, Copy, Debug, Default, Hash, Eq, PartialEq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct SeqTypes;

impl NodeType for SeqTypes {
    type Time = ViewNumber;
    type BlockHeader = Header;
    type BlockPayload = Payload;
    type SignatureKey = PubKey;
    type Transaction = Transaction;
    type InstanceState = NodeState;
    type ValidatedState = ValidatedState;
    type Membership = GeneralStaticCommittee<Self, PubKey>;
    type BuilderSignatureKey = FeeAccount;
}

pub type Leaf = hotshot_types::data::Leaf<SeqTypes>;
pub type Event = hotshot::types::Event<SeqTypes>;

pub type PubKey = BLSPubKey;
pub type PrivKey = <PubKey as SignatureKey>::PrivateKey;

pub type NetworkConfig = hotshot_orchestrator::config::NetworkConfig<PubKey>;

#[derive(Clone, Debug, Snafu, Deserialize, Serialize)]
pub enum Error {
    // TODO: Can we nest these errors in a `ValidationError` to group them?

    // Parent state commitment of block doesn't match current state commitment
    IncorrectParent,

    // New view number isn't strictly after current view
    IncorrectView,

    // Genesis block either has zero or more than one transaction
    GenesisWrongSize,

    // Genesis transaction not present in genesis block
    MissingGenesis,

    // Genesis transaction in non-genesis block
    UnexpectedGenesis,

    // Merkle tree error
    MerkleTreeError { error: String },

    BlockBuilding,
}

pub mod constants {
    pub use crate::v0_1::{
        BACKOFF_FACTOR, BACKOFF_JITTER, BLOCK_MERKLE_TREE_HEIGHT, FEE_MERKLE_TREE_HEIGHT,
        MAX_RETRY_DELAY, MIN_RETRY_DELAY, NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN,
        NUM_TXS_BYTE_LEN, TX_OFFSET_BYTE_LEN,
    };
}

pub mod traits {
    pub use crate::v0_1::{
        FromNsPayloadBytes, NsPayloadBytesRange, PersistenceOptions, SequencerPersistence,
        StateCatchup,
    };
}
