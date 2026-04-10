use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use derive_more::From;
use hotshot_types::traits::node_implementation::NodeType;
use jf_merkle_tree_compat::{
    DigestAlgorithm, Element, ForgetableMerkleTreeScheme, Index, MerkleCommitment, NodeValue,
    ToTraversalPath, prelude::MerkleProof,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use snafu::Snafu;
use tagged_base64::TaggedBase64;
use tide_disco::StatusCode;

use crate::QueryError;

/// This trait should be implemented by the MerkleTree that the API module is initialized for.
/// It defines methods utilized by the module.
pub trait MerklizedState<Types, const ARITY: usize>:
    ForgetableMerkleTreeScheme<Commitment = Self::Commit> + Send + Sync + Clone + 'static
where
    Types: NodeType,
{
    type Key: Index
        + Send
        + Sync
        + Serialize
        + ToTraversalPath<ARITY>
        + FromStr
        + DeserializeOwned
        + Display
        + CanonicalSerialize
        + CanonicalDeserialize;
    type Entry: Element
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + CanonicalSerialize
        + CanonicalDeserialize;
    type T: NodeValue + Send;
    type Commit: MerkleCommitment<Self::T>
        + Send
        + for<'a> TryFrom<&'a TaggedBase64>
        + Display
        + Debug
        + Into<TaggedBase64>;
    type Digest: DigestAlgorithm<Self::Entry, Self::Key, Self::T>;

    /// Retrieves the name of the state being queried.
    fn state_type() -> &'static str;

    /// Retrieves the field in the header containing the Merkle tree commitment
    /// for the state implementing this trait.
    fn header_state_commitment_field() -> &'static str;

    /// Get the height of the tree
    fn tree_height() -> usize;

    /// Insert a forgotten path into the tree.
    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, ARITY>,
    ) -> anyhow::Result<()>;
}

/// Errors surfaced to clients from a Merklized state API.
#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: tide_disco::RequestError,
    },
    #[snafu(display("{source}"))]
    Query {
        source: QueryError,
    },
    #[snafu(display("error {status}: {message}"))]
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BAD_REQUEST,
            Self::Query { source, .. } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}
