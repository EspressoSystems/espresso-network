//! This file contains the [`Request`] and [`Response`] traits. Any upstream
//! that wants to use the [`RequestResponseProtocol`] needs to implement these
//! traits for their specific types.

use std::fmt::Debug;

use anyhow::Result;

use super::Serializable;

/// A trait for a request. Associates itself with a response type.
#[cfg(not(test))]
pub trait Request: Send + Sync + Serializable + 'static + Clone + Debug {
    /// The response type associated with this request
    type Response: Send + Sync + Serializable + Clone + Debug;

    /// Validate the request, returning an error if it is not valid
    ///
    /// # Errors
    /// If the request is not valid
    fn validate(&self) -> Result<()>;
}

/// A trait for a request. Associates itself with a response type.
#[cfg(test)]
pub trait Request: Send + Sync + Serializable + 'static + Clone + Debug {
    /// The response type associated with this request
    type Response: Send + Sync + Serializable + Clone + Debug + PartialEq + Eq;

    /// Validate the request, returning an error if it is not valid
    ///
    /// # Errors
    /// If the request is not valid
    fn validate(&self) -> Result<()>;
}
