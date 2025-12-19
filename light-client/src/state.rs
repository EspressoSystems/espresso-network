//! Client-side state used to implement light client fetching and verification.

use std::borrow::Cow;

use anyhow::{ensure, Context, Result};
use espresso_types::{Leaf2, SeqTypes};
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    types::HeightIndexed,
};
use hotshot_types::{data::EpochNumber, PeerConfig};
use serde::{Deserialize, Serialize};

use crate::{
    client::Client,
    consensus::{
        leaf::LeafProofHint,
        quorum::{StakeTable, StakeTablePair, StakeTableQuorum},
    },
    storage::Storage,
};

/// Initial state for a [`LightClient`].
///
/// This [`Genesis`] forms the root of trust for a light client. It defines the initial stake table
/// which is used to verify blocks before the first epoch, which in turn allows it to verify
/// transitions to subsequent stake tables. Thus, this genesis must be configured correctly (i.e.
/// matching the genesis state of honest HotShot nodes) or else the light client may not operate
/// correctly.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Genesis {
    /// The number of blocks in an epoch.
    pub epoch_height: u64,

    /// The fixed stake table used before epochs begin.
    pub stake_table: Vec<PeerConfig<SeqTypes>>,
}

/// Client-side state required to implement the light client interface.
///
/// A [`LightClient`] can always be created [from scratch](Self::from_genesis), with no state, since
/// ultimately all data is fetched and verified from external query nodes. However, having some
/// persistent state can make it more efficient to use a [`LightClient`] over a long period of time,
/// as important artifacts can be cached locally, avoiding the need to frequently re-fetch and
/// verify them.
#[derive(Debug)]
pub struct LightClient<P, S> {
    db: P,
    server: S,
    epoch_height: u64,
    stake_table: StakeTable,
}

impl<P, S> LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    /// Create a light client from scratch, with no state.
    ///
    /// State will automatically be populated as queries are made. The provided genesis becomes the
    /// root of trust for verifying all state that is subsequently loaded by the light client. If
    /// the genesis is not correct (i.e. matching the genesis used by honest HotShot nodes) the
    /// light client may verify incorrect data, or fail to verify correct data.
    pub fn from_genesis(db: P, server: S, genesis: Genesis) -> Self {
        Self {
            db,
            server,
            epoch_height: genesis.epoch_height,
            stake_table: StakeTable::new(genesis.stake_table.into()),
        }
    }

    /// Fetch and verify the requested leaf.
    pub async fn fetch_leaf(&self, id: LeafId<SeqTypes>) -> Result<LeafQueryData<SeqTypes>> {
        let upper_bound = self.db.leaf_upper_bound(id).await?;
        let known_finalized = if let Some(upper_bound) = upper_bound {
            if leaf_matches_id(&upper_bound, id) {
                return Ok(upper_bound);
            }

            Some(upper_bound)
        } else {
            None
        };
        let known_finalized = known_finalized.as_ref().map(LeafQueryData::leaf);

        let proof = self
            .server
            .leaf_proof(id, known_finalized.map(Leaf2::height))
            .await?;
        let quorum;
        let hint = match proof.proof().epoch() {
            Some(epoch) => {
                quorum = StakeTableQuorum::new((epoch, self), self.epoch_height);
                LeafProofHint::Quorum(&quorum)
            },
            None => LeafProofHint::Assumption(known_finalized.context(
                "server returned proof with assumption, but we have no finalized upper bound to \
                 verify assumption",
            )?),
        };
        let leaf = proof.verify(hint).await?;

        // The server has given us a leaf and correctly proved it finalized, but we still need to
        // verify that it actually gave us the leaf we requested.
        ensure!(
            leaf_matches_id(&leaf, id),
            "server returned a valid leaf proof for the wrong leaf (requested leaf {id}, got leaf \
             {} with hash {})",
            leaf.height(),
            leaf.hash(),
        );

        // Having fetched and verified the leaf from the server, we can now cache it locally to
        // improve future requests.
        if let Err(err) = self.db.insert_leaf(leaf.clone()).await {
            // If this fails, we can still successfully return the leaf that we have in memory right
            // now, so this is just a warning.
            tracing::warn!("failed to cache fetched leaf: {err:#}");
        }

        Ok(leaf)
    }

    /// Fetch and verify the stake table for the requested epoch.
    pub async fn quorum_for_epoch(&self, _epoch: EpochNumber) -> Result<&StakeTable> {
        // TODO use dynamic stake table
        Ok(&self.stake_table)
    }
}

impl<P, S> StakeTablePair for (EpochNumber, &LightClient<P, S>)
where
    P: Storage,
    S: Client,
{
    async fn stake_table(&self) -> Result<Cow<'_, StakeTable>> {
        let stake_table = self.1.quorum_for_epoch(self.0).await?;
        Ok(Cow::Borrowed(stake_table))
    }

    async fn next_epoch_stake_table(&self) -> Result<Cow<'_, StakeTable>> {
        let stake_table = self.1.quorum_for_epoch(self.0 + 1).await?;
        Ok(Cow::Borrowed(stake_table))
    }
}

fn leaf_matches_id(leaf: &LeafQueryData<SeqTypes>, id: LeafId<SeqTypes>) -> bool {
    match id {
        LeafId::Number(h) => (h as u64) == leaf.height(),
        LeafId::Hash(h) => h == leaf.hash(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{storage::SqliteStorage, testing::TestClient};

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaf_twice() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis(),
        );

        // Fetch the leaf for the first time. We will need to get it from the server.
        let leaf = client.remember_leaf(1).await;
        assert_eq!(lc.fetch_leaf(LeafId::Number(1)).await.unwrap(), leaf);

        // Fetching the leaf again hits the cache.
        client.forget_leaf(1).await;
        assert_eq!(lc.fetch_leaf(LeafId::Number(1)).await.unwrap(), leaf);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaf_upper_bound() {
        let client = TestClient::default();

        let db = SqliteStorage::default().await.unwrap();
        db.insert_leaf(client.leaf(2).await).await.unwrap();

        let lc = LightClient::from_genesis(db, client.clone(), client.genesis());
        assert_eq!(
            lc.fetch_leaf(LeafId::Number(1)).await.unwrap(),
            client.leaf(1).await,
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaf_invalid_proof() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis(),
        );
        client.return_invalid_proof(1).await;
        lc.fetch_leaf(LeafId::Number(1)).await.unwrap_err();
        lc.fetch_leaf(LeafId::Hash(client.leaf(1).await.hash()))
            .await
            .unwrap_err();
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaf_wrong_leaf() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis(),
        );
        client.return_wrong_leaf(1, 2).await;
        lc.fetch_leaf(LeafId::Number(1)).await.unwrap_err();
        lc.fetch_leaf(LeafId::Hash(client.leaf(1).await.hash()))
            .await
            .unwrap_err();
    }
}
