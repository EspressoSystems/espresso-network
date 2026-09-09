use anyhow::Result;
use async_trait::async_trait;
use hotshot_types::traits::signature_key::SignatureKey;

use super::request::Request;

/// A trait that allows the [`RequestResponseProtocol`] to get the recipients that a specific message should
/// expect responses from. In `HotShot` this would go on top of the [`Membership`] trait and determine
/// which nodes are able (quorum/DA) to respond to which requests
#[async_trait]
pub trait RecipientSource<R: Request, K: SignatureKey + 'static>: Send + Sync + 'static {
    /// Get all the recipients that the specific request should expect responses from
    async fn get_expected_responders(&self, request: &R) -> Result<Vec<K>>;
}
