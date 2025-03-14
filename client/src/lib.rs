use std::time::Duration;

use anyhow::Context;
use espresso_types::{
    config::PublicNetworkConfig, FeeAccount, FeeAmount, FeeMerkleTree, Header, PubKey,
};
use ethers::types::Address;
use futures::{stream::BoxStream, StreamExt};
use hotshot_types::stake_table::StakeTableEntry;
use jf_merkle_tree::{
    prelude::{MerkleProof, Sha3Node},
    MerkleTreeScheme,
};
use surf_disco::{
    error::ClientError,
    socket::{Connection, Unsupported},
    Url,
};
use tokio::time::sleep;
use vbs::version::StaticVersion;

pub type SequencerApiVersion = StaticVersion<0, 1>;

#[derive(Clone, Debug)]
pub struct SequencerClient(surf_disco::Client<ClientError, SequencerApiVersion>);

pub type FeeMerkleProof = MerkleProof<FeeAmount, FeeAccount, Sha3Node, { FeeMerkleTree::ARITY }>;

impl SequencerClient {
    pub fn new(provider: Url) -> Self {
        Self(surf_disco::Client::new(provider))
    }

    /// GET Block Height from the node
    pub async fn get_height(&self) -> anyhow::Result<u64> {
        self.0
            .get::<u64>("node/block-height")
            .send()
            .await
            .context("getting Espresso block height")
    }

    /// Get the Number of Transactions
    pub async fn get_transaction_count(&self) -> anyhow::Result<u64> {
        self.0
            .get::<u64>("node/transactions/count")
            .send()
            .await
            .context("getting Espresso transaction count")
    }

    /// Subscribe to a stream of Block Headers
    pub async fn subscribe_headers(
        &self,
        height: u64,
    ) -> anyhow::Result<BoxStream<'static, Result<Header, ClientError>>> {
        self.0
            .socket(&format!("availability/stream/headers/{height}"))
            .subscribe::<Header>()
            .await
            .context("subscribing to Espresso headers")
            .map(|s| s.boxed())
    }

    /// Subscribe to a stream of Block Headers
    pub async fn subscribe_blocks(
        &self,
        height: u64,
    ) -> anyhow::Result<Connection<Header, Unsupported, ClientError, SequencerApiVersion>> {
        self.0
            .socket(&format!("availability/stream/blocks/{height}"))
            .subscribe()
            .await
            .context("subscribing to Espresso Blocks")
    }

    /// Get the balance for a given account at a given block height, defaulting to current balance.
    pub async fn get_espresso_balance(
        &self,
        address: Address,
        block: Option<u64>,
    ) -> anyhow::Result<FeeAmount> {
        // Get the block height to query at, defaulting to the latest block.
        let mut block = match block {
            Some(block) => block,
            None => self.get_height().await?,
        };
        // As of block zero the state is empty, and the balance will be zero.
        if block == 0 {
            return Ok(0.into());
        }
        // Block is non-zero, we can safely decrement to query the state as of the previous block.
        block -= 1;
        // Download the Merkle path for this fee account at the specified block height. Transient errors
        // are possible (for example, if we are fetching from the latest block, the block height might
        // get incremented shortly before the state becomes available) so retry a few times.
        let mut retry = 0;
        let max_retries = 10;
        let proof = loop {
            tracing::debug!(%address, block, retry, "fetching Espresso balance");
            match self
                .0
                .get::<FeeMerkleProof>(&format!("fee-state/{block}/{address:#x}"))
                .send()
                .await
            {
                Ok(proof) => break proof,
                Err(err) => {
                    tracing::warn!("error getting account balance: {err:#}");
                    retry += 1;

                    if retry == max_retries {
                        return Err(err).context("getting account balance");
                    } else {
                        sleep(Duration::from_millis(200)).await;
                    }
                },
            }
        };

        // If the element in the Merkle path is missing -- there is no account with this address -- the
        // balance is defined to be 0.
        let balance = proof.elem().copied().unwrap_or(0.into());
        Ok(balance)
    }

    pub async fn current_epoch(&self) -> anyhow::Result<Option<u64>> {
        self.0
            .get::<Option<u64>>("node/current_epoch")
            .send()
            .await
            .context("getting epoch value")
    }

    pub async fn stake_table(&self, epoch: u64) -> anyhow::Result<Vec<StakeTableEntry<PubKey>>> {
        self.0
            .get::<_>(&format!("node/stake-table/{epoch}"))
            .send()
            .await
            .context("getting stake table")
    }

    pub async fn da_members(&self, epoch: u64) -> anyhow::Result<Vec<StakeTableEntry<PubKey>>> {
        self.0
            .get::<_>(&format!("node/stake-table/da/{epoch}"))
            .send()
            .await
            .context("getting da stake table")
    }

    pub async fn config(&self) -> anyhow::Result<PublicNetworkConfig> {
        self.0
            .get::<PublicNetworkConfig>("config/hotshot")
            .send()
            .await
            .context("getting hotshot config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Regression test for a bug where the block number underflowed. This test would panic
    // on the previous implementation, as long as overflow checks are enabled.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_regression_block_number_underflow() {
        let client = SequencerClient::new("http://dummy-url:3030".parse().unwrap());
        assert_eq!(
            client
                .get_espresso_balance(Address::zero(), Some(0))
                .await
                .unwrap(),
            0.into()
        )
    }
}
