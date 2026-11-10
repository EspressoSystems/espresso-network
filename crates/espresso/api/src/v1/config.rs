//! V1 config API.

use async_trait::async_trait;
use espresso_types::config::PublicNetworkConfig;

/// The node's runtime configuration. Its type is built from the node binary's CLI options, which
/// this crate cannot depend on, so it is erased. It serializes exactly like the value it holds.
pub type RuntimeConfig = Box<dyn erased_serde::Serialize + Send + Sync>;

#[async_trait]
pub trait ConfigApi {
    async fn hotshot_config(&self) -> anyhow::Result<PublicNetworkConfig>;

    /// The allowlisted environment variables, each as a single `KEY=value` string. Unset
    /// variables are present with an empty value.
    async fn env(&self) -> anyhow::Result<Vec<String>>;

    async fn runtime_config(&self) -> anyhow::Result<RuntimeConfig>;
}
