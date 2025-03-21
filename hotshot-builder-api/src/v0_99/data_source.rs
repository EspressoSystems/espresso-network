use async_trait::async_trait;
use hotshot_types::{bundle::Bundle, data::VidCommitment, traits::node_implementation::NodeType};

use super::builder::BuildError;
/// No changes to these types
pub use crate::v0_1::data_source::AcceptsTxnSubmits;

#[async_trait]
pub trait BuilderDataSource<TYPES: NodeType> {
    /// To get the list of available blocks
    async fn bundle(
        &self,
        parent_view: u64,
        parent_hash: &VidCommitment,
        view_number: u64,
    ) -> Result<Bundle<TYPES>, BuildError>;

    /// To get the builder's address
    async fn builder_address(&self) -> Result<TYPES::BuilderSignatureKey, BuildError>;
}
