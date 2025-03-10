//! This file contains the [`Request`] and [`Response`] traits. Any upstream
//! that wants to use the [`RequestResponseProtocol`] needs to implement these
//! traits for their specific types.

use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;

use super::Serializable;

/// A trait for a request. Associates itself with a response type.
#[async_trait]
pub trait Request: Send + Sync + Serializable + 'static + Clone + Debug {
    /// The response type associated with this request
    type Response: Response<Self>;

    /// Validate the request, returning an error if it is not valid
    ///
    /// # Errors
    /// If the request is not valid
    async fn validate(&self) -> Result<()>;
}

/// A trait that a response needs to implement
#[async_trait]
#[cfg(not(test))]
pub trait Response<R: Request>: Send + Sync + Serializable + Clone + Debug {
    /// Validate the response, making sure it is valid for the given request
    ///
    /// # Errors
    /// If the response is not valid for the given request
    async fn validate(&self, request: &R) -> Result<()>;
}

/// A trait that a response needs to implement
#[async_trait]
#[cfg(test)]
pub trait Response<R: Request>:
    Send + Sync + Serializable + Clone + Debug + PartialEq + Eq
{
    /// Validate the response, making sure it is valid for the given request
    ///
    /// # Errors
    /// If the response is not valid for the given request
    async fn validate(&self, request: &R) -> Result<()>;
}
