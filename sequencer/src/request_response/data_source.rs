//! This file contains the [`DataSource`] trait. This trait allows the [`RequestResponseProtocol`]
//! to calculate/derive a response for a specific request. In the confirmation layer the implementer
//! would be something like a [`FeeMerkleTree`] for fee catchup

use anyhow::Result;
use async_trait::async_trait;
use request_response::data_source::DataSource as DataSourceTrait;

use super::request::{Request, Response};

#[derive(Clone, Debug)]
pub struct DataSource {}

/// Implement the trait that allows the [`RequestResponseProtocol`] to calculate/derive a response for a specific request
#[async_trait]
impl DataSourceTrait<Request> for DataSource {
    async fn derive_response_for(&self, request: &Request) -> Result<Response> {
        match request {
            Request::Example => Ok(Response::Example),
        }
    }
}
