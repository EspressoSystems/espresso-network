//! Core API trait definition

use async_trait::async_trait;
use serialization_api::v1::{NamespaceProofQueryData, ViewNumber};

/// Node API trait defining the core business logic
#[async_trait]
pub trait NodeApi {
    /// Get the current view number
    async fn get_view_number(&self) -> anyhow::Result<ViewNumber>;

    /// Get namespace proof and transactions by height and namespace ID
    async fn get_namespace_proof(
        &self,
        height: u64,
        namespace: u64,
    ) -> anyhow::Result<NamespaceProofQueryData>;
}

/// State struct for the node API
///
/// This currently has a dummy implementation returning hardcoded values.
/// The real implementation will eventually live in crates/espresso/node.
#[derive(Clone, Default)]
pub struct NodeApiState;

#[async_trait]
impl NodeApi for NodeApiState {
    async fn get_view_number(&self) -> anyhow::Result<ViewNumber> {
        Ok(ViewNumber { value: 1 })
    }

    async fn get_namespace_proof(
        &self,
        _height: u64,
        _namespace: u64,
    ) -> anyhow::Result<NamespaceProofQueryData> {
        Ok(NamespaceProofQueryData {
            proof: None,
            transactions: vec![],
        })
    }
}
