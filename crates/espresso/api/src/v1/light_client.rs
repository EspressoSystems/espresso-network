//! V1 light-client API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/light-client.toml`.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum LeafQuery {
    Height(u64),
    Hash(String),
    BlockHash(String),
    PayloadHash(String),
}

#[derive(Debug, Clone)]
pub enum HeaderQuery {
    Height(u64),
    Hash(String),
    PayloadHash(String),
}

#[async_trait]
pub trait LightClientApi {
    type LeafProof: Serialize + Send + Sync + 'static;
    type HeaderProof: Serialize + Send + Sync + 'static;
    type StakeTableEvents: Serialize + Send + Sync + 'static;
    type PayloadProof: Serialize + Send + Sync + 'static;
    type NamespaceProof: Serialize + Send + Sync + 'static;

    async fn get_leaf_proof(
        &self,
        query: LeafQuery,
        finalized: Option<u64>,
    ) -> anyhow::Result<Self::LeafProof>;

    async fn get_header_proof(
        &self,
        root: u64,
        requested: HeaderQuery,
    ) -> anyhow::Result<Self::HeaderProof>;

    async fn get_light_client_stake_table(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Self::StakeTableEvents>;

    async fn get_payload_proof(&self, height: u64) -> anyhow::Result<Self::PayloadProof>;

    async fn get_payload_proof_range(
        &self,
        start: u64,
        end: u64,
    ) -> anyhow::Result<Vec<Self::PayloadProof>>;

    async fn get_lc_namespace_proof(
        &self,
        height: u64,
        namespace: u64,
    ) -> anyhow::Result<Self::NamespaceProof>;

    async fn get_lc_namespace_proof_range(
        &self,
        start: u64,
        end: u64,
        namespace: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>>;

    /// `namespaces` is the raw `TaggedBase64`-encoded path segment produced by the light-client
    /// client (tag `NS`, wrapping a JSON `Vec<u64>`); decoding it is left to the implementation
    /// so this crate does not need a `tagged-base64` dependency.
    async fn get_lc_namespaces_proof_range(
        &self,
        start: u64,
        end: u64,
        namespaces: String,
    ) -> anyhow::Result<Vec<HashMap<u64, Self::NamespaceProof>>>;
}
