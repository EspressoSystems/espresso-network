//! This file contains the [`DataSource`] trait. This trait allows the [`RequestResponseProtocol`]
//! to calculate/derive a response for a specific request. In the confirmation layer the implementer
//! would be something like a [`FeeMerkleTree`] for fee catchup

use anyhow::Result;
use async_trait::async_trait;

use super::request::Request;

/// The trait that allows the [`RequestResponseProtocol`] to calculate/derive a response for a specific request
#[async_trait]
pub trait DataSource<R: Request>: Send + Sync + 'static + Clone {
    /// Calculate/derive the response for a specific request
    async fn derive_response_for(&self, request: &R) -> Result<R::Response>;
}
