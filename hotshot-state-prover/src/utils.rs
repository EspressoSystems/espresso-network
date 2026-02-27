use std::time::Duration;

use alloy::providers::Provider;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct ChainIdRetry {
    pub request_timeout: Duration,
    pub global_timeout: Duration,
    pub retry_delay: Duration,
}

impl Default for ChainIdRetry {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(2),
            global_timeout: Duration::from_secs(30),
            retry_delay: Duration::from_secs(1),
        }
    }
}

impl ChainIdRetry {
    /// Attempts to get the chain ID from the provider, retrying on failure.
    ///
    /// Combined with our `SwitchingTransport` this can be used to query
    /// the chain ID from multiple providers.
    ///
    /// Each individual request is wrapped with a timeout to handle hanging requests.
    /// The entire retry loop is wrapped with the global timeout to ensure we never
    /// exceed the specified duration.
    ///
    /// Returns an error if unable to get chain ID after the global timeout period.
    pub async fn get_chain_id(self, provider: &impl Provider) -> Result<u64> {
        tokio::time::timeout(self.global_timeout, async {
            loop {
                match tokio::time::timeout(self.request_timeout, provider.get_chain_id()).await {
                    Ok(Ok(id)) => return Ok(id),
                    Ok(Err(err)) => {
                        tracing::warn!("Failed to get chain ID, retrying: {err}");
                    },
                    Err(_) => {
                        tracing::warn!(
                            "Request timed out after {:?}, retrying",
                            self.request_timeout
                        );
                    },
                }
                tokio::time::sleep(self.retry_delay).await;
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("Failed to get chain ID after {:?}", self.global_timeout))?
    }
}

#[cfg(test)]
mod tests {
    use alloy::{node_bindings::Anvil, providers::ProviderBuilder, rpc::client::RpcClient};
    use espresso_types::{v0_1::SwitchingTransport, L1ClientOptions};
    use rstest::rstest;

    use super::*;

    #[tokio::test]
    async fn test_get_chain_id_switches_to_good_url() {
        let anvil = Anvil::new().spawn();

        let transport = SwitchingTransport::new(
            L1ClientOptions::default(),
            vec!["http://localhost:1".parse().unwrap(), anvil.endpoint_url()],
        )
        .expect("failed to create switching transport");
        let rpc_client = RpcClient::new(transport, false);
        let provider = ProviderBuilder::new().connect_client(rpc_client);

        let chain_id = ChainIdRetry::default()
            .get_chain_id(&provider)
            .await
            .unwrap();
        assert_eq!(chain_id, anvil.chain_id());
    }

    #[rstest]
    #[case::only_bad_urls(vec!["http://localhost:1", "http://localhost:2"])]
    // A non-routable IP address to simulate a hanging request:
    #[case::hanging_requests(vec!["http://10.255.255.1:1"])]
    #[tokio::test]
    async fn test_get_chain_id_failure(#[case] urls: Vec<&str>) {
        let parsed_urls: Vec<_> = urls.iter().map(|u| u.parse().unwrap()).collect();
        let transport = SwitchingTransport::new(L1ClientOptions::default(), parsed_urls)
            .expect("failed to create switching transport");
        let rpc_client = RpcClient::new(transport, false);
        let provider = ProviderBuilder::new().connect_client(rpc_client);

        let result = ChainIdRetry {
            global_timeout: Duration::from_secs(1),
            ..Default::default()
        }
        .get_chain_id(&provider)
        .await;

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to get chain ID"));
    }
}
