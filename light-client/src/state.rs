//! Client-side state used to implement light client fetching and verification.

use std::{collections::BTreeMap, future::Future, sync::Arc};

use anyhow::{ensure, Context, Result};
use async_lock::RwLock;
use committable::Committable;
use espresso_types::{
    select_active_validator_set, Header, Leaf2, NamespaceId, PubKey, SeqTypes, StakeTableState,
    Transaction,
};
use hotshot_query_service::{
    availability::{BlockQueryData, LeafId, LeafQueryData, PayloadQueryData},
    node::BlockId,
    types::HeightIndexed,
};
use hotshot_types::{
    data::EpochNumber, stake_table::StakeTableEntry, traits::node_implementation::ConsensusTime,
    utils::root_block_in_epoch,
};
use serde::{Deserialize, Serialize};

use crate::{
    client::Client,
    consensus::{
        leaf::LeafProofHint,
        quorum::{Quorum, StakeTable, StakeTablePair, StakeTableQuorum},
    },
    storage::{LeafRequest, Storage},
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

    /// The first epoch where the stake table came from the contract, rather than the genesis stake
    /// table.
    pub first_epoch_with_dynamic_stake_table: EpochNumber,

    /// The fixed stake table used before epochs begin.
    pub stake_table: Vec<StakeTableEntry<PubKey>>,
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Maximum number of stake tables to cache in memory at any given time.
    pub num_stake_tables: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            num_stake_tables: 100,
        }
    }
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
    opt: Options,

    epoch_height: u64,
    first_epoch_with_dynamic_stake_table: EpochNumber,
    genesis_stake_table: Arc<StakeTable>,

    // We cache stake tables in memory since they are large and expensive to load from the database.
    stake_tables: RwLock<BTreeMap<EpochNumber, Arc<StakeTable>>>,
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
        Self::from_genesis_with_options(db, server, genesis, Default::default())
    }

    /// Create a light client from scratch, with no state, using the given options.
    ///
    /// State will automatically be populated as queries are made. The provided genesis becomes the
    /// root of trust for verifying all state that is subsequently loaded by the light client. If
    /// the genesis is not correct (i.e. matching the genesis used by honest HotShot nodes) the
    /// light client may verify incorrect data, or fail to verify correct data.
    pub fn from_genesis_with_options(db: P, server: S, genesis: Genesis, opt: Options) -> Self {
        Self {
            db,
            server,
            opt,
            epoch_height: genesis.epoch_height,
            genesis_stake_table: Arc::new(genesis.stake_table.into()),
            first_epoch_with_dynamic_stake_table: genesis.first_epoch_with_dynamic_stake_table,
            stake_tables: Default::default(),
        }
    }

    /// Fetch and verify the requested leaf.
    pub async fn fetch_leaf(&self, id: LeafId<SeqTypes>) -> Result<LeafQueryData<SeqTypes>> {
        self.fetch_leaf_with_quorum(id, |epoch| {
            StakeTableQuorum::new((epoch, self), self.epoch_height)
        })
        .await
    }

    async fn fetch_leaf_with_quorum<Q>(
        &self,
        id: LeafId<SeqTypes>,
        quorum: impl Send + FnOnce(EpochNumber) -> Q,
    ) -> Result<LeafQueryData<SeqTypes>>
    where
        Q: Send + Quorum,
    {
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
        self.fetch_leaf_from_server(id, known_finalized, quorum)
            .await
    }

    /// Fetches leaves in range [start_height, end_height)
    pub async fn fetch_leaves_in_range(
        &self,
        start_height: usize,
        end_height: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        ensure!(
            start_height < end_height,
            "invalid range: start must be < end"
        );

        // first try to fetch all the leaves from the local database
        let leaves = self
            .db
            .get_leaves_in_range(start_height as u32, end_height as u32)
            .await?;

        if leaves.len() == end_height - start_height {
            // we have all the leaves in the range
            return Ok(leaves);
        }

        // Fetch the last leaf in the range as our known finalized anchor point
        let known_end_leaf = self.fetch_leaf(LeafId::Number(end_height - 1)).await?;

        // at this point, we know the end leaf is valid and is finalized
        // now we need to fetch all leaves from start to end - 1 from the server
        let leaves = self.fetch_leaves_in_range_from_server(start_height, end_height - 1, &known_end_leaf)
            .await
        // add the known end leaf to the result
        .map(|mut leaves| {
            leaves.push(known_end_leaf);
            leaves
        })?;
        Ok(leaves)
    }

    /// Fetches headers in range [start_height, end_height)
    pub async fn fetch_headers_in_range(
        &self,
        start_height: usize,
        end_height: usize,
    ) -> Result<Vec<Header>> {
        ensure!(
            start_height < end_height,
            "invalid range: start must be < end"
        );

        // Reuse the verified leaf path to guarantee header correctness.
        let leaves = self.fetch_leaves_in_range(start_height, end_height).await?;
        Ok(leaves
            .into_iter()
            .map(|leaf| leaf.header().clone())
            .collect())
    }

    /// Fetches leaves from the server in range [start_height, end_height) and verifies them by
    /// walking backwards from the known finalized leaf, ensuring each leaf's hash matches the
    /// parent commitment of the subsequent leaf.
    async fn fetch_leaves_in_range_from_server(
        &self,
        start_height: usize,
        end_height: usize,
        known_finalized: &LeafQueryData<SeqTypes>,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        // we will fetch all the leaves till the known finalized leaf
        let leaves = self
            .server
            // `get_leaves_in_range` is exclusive of the end height
            // which we dont need because we already know the end leaf
            .get_leaves_in_range(start_height, end_height)
            .await?;

        ensure!(
            leaves.len() == end_height.saturating_sub(start_height),
            "server returned {} leaves for range [{}, {})",
            leaves.len(),
            start_height,
            end_height
        );

        // Walk backwards from the known finalized leaf, ensuring each parent hash matches
        let mut expected_parent = known_finalized.leaf().parent_commitment();
        for leaf in leaves.iter().rev() {
            let leaf_hash = leaf.hash();
            ensure!(
                leaf_hash == expected_parent,
                "leaf hash mismatch: expected parent hash {:?}, got leaf hash {:?}",
                expected_parent,
                leaf_hash
            );
            expected_parent = leaf.leaf().parent_commitment();
        }

        // Cache the fetched leaves, but still return them even if caching fails
        for leaf in &leaves {
            if let Err(err) = self.db.insert_leaf(leaf.clone()).await {
                tracing::warn!(
                    "failed to cache leaf at height {}: {:#?}",
                    leaf.height(),
                    err
                )
            }
        }

        Ok(leaves)
    }

    /// Fetch and verify the requested header.
    pub async fn fetch_header(&self, id: BlockId<SeqTypes>) -> Result<Header> {
        self.fetch_header_with_quorum(id, |epoch| {
            StakeTableQuorum::new((epoch, self), self.epoch_height)
        })
        .await
    }

    pub async fn fetch_headers_in_range(
        &self,
        start_height: usize,
        end_height: usize,
    ) -> Result<Vec<Header>> {
        todo!()
    }

    async fn fetch_header_with_quorum<Q>(
        &self,
        id: BlockId<SeqTypes>,
        quorum: impl Send + FnOnce(EpochNumber) -> Q,
    ) -> Result<Header>
    where
        Q: Send + Quorum,
    {
        if let Some(leaf) = self.db.leaf_upper_bound(id).await? {
            if leaf_matches_id(&leaf, id) {
                // If we have the leaf for the requested header in our database already, we can just
                // extract the header.
                return Ok(leaf.header().clone());
            } else {
                // Otherwise, if we have a leaf that is known to be greater than the requested
                // header, we can ask the server for an inclusion proof for the requested header
                // relative to the Merkle root in the upper bound leaf.
                let proof = self.server.header_proof(leaf.height(), id).await?;
                let header = proof
                    .verify(leaf.header().block_merkle_tree_root())
                    .context("invalid header proof")?;

                // The server has given us a header and correctly proved it finalized, but we still
                // need to verify that it actually gave us the header we requested.
                ensure!(
                    header_matches_id(&header, id),
                    "server returned a valid header proof for the wrong header (requested header \
                     {id}, got header {} with hash {})",
                    header.height(),
                    header.commit(),
                );

                return Ok(header);
            }
        }

        // We have neither the requested header nor an upper bound for it. All we can do is fetch
        // the corresponding leaf from the server (verifying a leaf proof) and then extract the
        // header from there.
        let leaf = self.fetch_leaf_from_server(id, None, quorum).await?;
        Ok(leaf.header().clone())
    }

    /// Fetch and verify the requested payload.
    pub async fn fetch_payload(&self, id: BlockId<SeqTypes>) -> Result<PayloadQueryData<SeqTypes>> {
        Ok(self.fetch_block(id).await?.into())
    }

    /// Fetch and verify the requested block.
    pub async fn fetch_block(&self, id: BlockId<SeqTypes>) -> Result<BlockQueryData<SeqTypes>> {
        let header = self.fetch_header(id).await?;
        let proof = self.server.payload_proof(header.height()).await?;
        let payload = proof.verify(&header)?;
        Ok(BlockQueryData::new(header, payload))
    }

    /// Fetch and verify the transactions in the given namespace of the requested block.
    pub async fn fetch_namespace(
        &self,
        id: BlockId<SeqTypes>,
        namespace: NamespaceId,
    ) -> Result<Vec<Transaction>> {
        let header = self.fetch_header(id).await?;
        let proof = self
            .server
            .namespace_proof(header.height(), namespace)
            .await?;
        proof.verify(&header, namespace)
    }

    /// Fetch and verify the transactions in the given namespace of blocks in the range
    /// `[start_height, end_height)`.
    pub async fn fetch_namespaces_in_range(
        &self,
        start_height: usize,
        end_height: usize,
        namespace: NamespaceId,
    ) -> Result<Vec<Vec<Transaction>>> {
        let headers = self
            .fetch_headers_in_range(start_height, end_height)
            .await?;
        let proofs = self
            .server
            .namespace_proofs_in_range(start_height as u64, end_height as u64, namespace)
            .await?;
        ensure!(
            proofs.len() == headers.len(),
            "server returned wrong number of namespace proofs (expected {}, got {})",
            headers.len(),
            proofs.len()
        );
        proofs
            .into_iter()
            .zip(headers)
            .map(|(proof, header)| proof.verify(&header, namespace))
            .collect()
    }

    /// Fetch and verify the stake table for the requested epoch.
    pub async fn quorum_for_epoch(&self, epoch: EpochNumber) -> Result<Arc<StakeTable>> {
        if epoch < self.first_epoch_with_dynamic_stake_table {
            return Ok(self.genesis_stake_table.clone());
        }

        // Check cache for the desired stake table.
        {
            let cache = self.stake_tables.read().await;
            if let Some(stake_table) = cache.get(&epoch) {
                tracing::debug!(%epoch, "found stake table in cache");
                return Ok(stake_table.clone());
            }
        }

        // If we didn't find the exact stake table we are looking for in cache, look for it in our
        // local database, or an earlier one we can catch up from.
        let (lower_bound, mut stake_table, mut prev_quorum) =
            if let Some((lower_bound, stake_table)) = self.db.stake_table_lower_bound(epoch).await?
            {
                if lower_bound == epoch {
                    // We have the exact quorum we requested already in our database. Add it to cache
                    // and return it.
                    tracing::debug!(%epoch, "found stake table in database");
                    let quorum = stake_table_state_to_quorum(stake_table)?;
                    return Ok(self.cache_stake_table(epoch, Arc::new(quorum)).await);
                }

                (
                    lower_bound,
                    stake_table.clone(),
                    Arc::new(stake_table_state_to_quorum(stake_table)?),
                )
            } else {
                // We don't have any stake table earlier than `epoch` as a starting point, so we must
                // start from the genesis state.
                (
                    self.first_epoch_with_dynamic_stake_table - 1,
                    StakeTableState::default(),
                    self.genesis_stake_table.clone(),
                )
            };
        tracing::info!(from = %lower_bound, to = %epoch, "performing stake table catchup");

        // Replay one epoch at a time from the lower bound stake table to the requested epoch.
        for epoch in *lower_bound + 1..=*epoch {
            let events = self
                .server
                .stake_table_events(EpochNumber::new(epoch))
                .await?;
            tracing::debug!(epoch, num_events = events.len(), "reconstruct stake table");
            for event in events {
                tracing::debug!(epoch, ?event, "replay event");
                if let Err(err) = stake_table.apply_event(event).context("applying event")? {
                    tracing::warn!("allowed error in event: {err:#}");
                }
            }
            let next_quorum = Arc::new(stake_table_state_to_quorum(stake_table.clone())?);

            // Since we are reconstructing based on events from an untrusted server, we need to
            // compare the hash of the stake table after each epoch to the hash recorded in the
            // epoch root header, which is certified by the previous stake table.
            let root_height = root_block_in_epoch(epoch - 1, self.epoch_height);
            let root = self
                .fetch_header_with_quorum(BlockId::Number(root_height as usize), |_| {
                    StakeTableQuorum::new((prev_quorum, next_quorum.clone()), self.epoch_height)
                })
                .await
                .context("fetching epoch root for {epoch}")?;
            let hash = root.next_stake_table_hash().context(format!(
                "epoch {epoch} root {root_height} does not have next stake table hash"
            ))?;
            ensure!(
                hash == stake_table.commit(),
                "epoch {epoch} root {root_height} stake table hash {hash} does not match \
                 reconstructed hash {}",
                stake_table.commit(),
            );

            prev_quorum = next_quorum;
        }

        // Finally, add the reconstructed stake table to cache and storage, and return it.
        if let Err(err) = self.db.insert_stake_table(epoch, &stake_table).await {
            // If this fails, we can still successfully return the stake table that we have in
            // memory right now, so this is just a warning.
            tracing::warn!(%epoch, "failed to cache stake table: {err:#}");
        }
        Ok(self.cache_stake_table(epoch, prev_quorum).await)
    }

    fn fetch_leaf_from_server<'a, 'b, Q>(
        &'a self,
        id: impl Send + Into<LeafRequest> + 'a,
        known_finalized: Option<&'b Leaf2>,
        make_quorum: impl 'a + Send + FnOnce(EpochNumber) -> Q,
    ) -> impl 'b + Send + Future<Output = Result<LeafQueryData<SeqTypes>>>
    where
        'a: 'b,
        Q: Send + Quorum,
    {
        async move {
            let id = id.into();
            let proof = self
                .server
                .leaf_proof(id, known_finalized.map(Leaf2::height))
                .await?;
            let quorum;
            let hint = match proof.proof().epoch() {
                Some(epoch) => {
                    quorum = make_quorum(epoch);
                    LeafProofHint::Quorum(&quorum)
                },
                None => LeafProofHint::Assumption(known_finalized.context(
                    "server returned proof with assumption, but we have no finalized upper bound \
                     to verify assumption",
                )?),
            };
            let leaf = proof.verify(hint).await?;

            // The server has given us a leaf and correctly proved it finalized, but we still need to
            // verify that it actually gave us the leaf we requested.
            ensure!(
                leaf_matches_id(&leaf, id),
                "server returned a valid leaf proof for the wrong leaf (requested leaf {id}, got \
                 leaf {} with hash {})",
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
    }

    async fn cache_stake_table(
        &self,
        epoch: EpochNumber,
        stake_table: Arc<StakeTable>,
    ) -> Arc<StakeTable> {
        let mut cache = self.stake_tables.write().await;

        // If inserting the new stake table would cause the cache to exceed its maximum size, first
        // delete an old stake table.
        if cache.len() >= self.opt.num_stake_tables {
            // Always delete the _second oldest_ stake table. We want to keep the oldest around
            // because it is the hardest to catch up for if we need it again (we would have to go
            // all the way back to genesis). The second oldest is the least likely to be used again
            // after the oldest, while still being easy to replay if we do need it (because we can
            // just replay from the cached oldest).
            if let Some(&second_oldest_epoch) = cache.keys().nth(1) {
                cache.remove(&second_oldest_epoch);
            }
        }

        cache.entry(epoch).insert_entry(stake_table).get().clone()
    }
}

fn stake_table_state_to_quorum(state: StakeTableState) -> Result<StakeTable> {
    let validators = state.into_validators();
    let active_validators = select_active_validator_set(&validators)?;
    Ok(active_validators
        .into_values()
        .map(|validator| StakeTableEntry {
            stake_key: validator.stake_table_key,
            stake_amount: validator.stake,
        })
        .collect())
}

impl<P, S> StakeTablePair for (EpochNumber, &LightClient<P, S>)
where
    P: Storage,
    S: Client,
{
    async fn stake_table(&self) -> Result<Arc<StakeTable>> {
        self.1.quorum_for_epoch(self.0).await
    }

    async fn next_epoch_stake_table(&self) -> Result<Arc<StakeTable>> {
        self.1.quorum_for_epoch(self.0 + 1).await
    }
}

fn leaf_matches_id(leaf: &LeafQueryData<SeqTypes>, id: impl Into<LeafRequest>) -> bool {
    match id.into() {
        LeafRequest::Leaf(LeafId::Number(h)) | LeafRequest::Header(BlockId::Number(h)) => {
            (h as u64) == leaf.height()
        },
        LeafRequest::Leaf(LeafId::Hash(h)) => h == leaf.hash(),
        LeafRequest::Header(BlockId::Hash(h)) => h == leaf.block_hash(),
        LeafRequest::Header(BlockId::PayloadHash(h)) => h == leaf.payload_hash(),
    }
}

fn header_matches_id(header: &Header, id: BlockId<SeqTypes>) -> bool {
    match id {
        BlockId::Number(n) => header.height() == (n as u64),
        BlockId::Hash(h) => header.commit() == h,
        BlockId::PayloadHash(h) => header.payload_commitment() == h,
    }
}

#[cfg(test)]
mod test {
    use espresso_types::NsIndex;
    use hotshot_query_service::availability::TransactionIndex;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{storage::SqliteStorage, testing::TestClient};

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaf_twice() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
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

        let lc = LightClient::from_genesis(db, client.clone(), client.genesis().await);
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
            client.genesis().await,
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
            client.genesis().await,
        );
        client.return_wrong_leaf(1, 2).await;
        lc.fetch_leaf(LeafId::Number(1)).await.unwrap_err();
        lc.fetch_leaf(LeafId::Hash(client.leaf(1).await.hash()))
            .await
            .unwrap_err();
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_header_twice() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        // Fetch the header for the first time. We will need to get it from the server.
        let leaf = client.remember_leaf(1).await;
        assert_eq!(
            lc.fetch_header(BlockId::Number(1)).await.unwrap(),
            *leaf.header()
        );

        // Fetching the header again hits the cache.
        client.forget_leaf(1).await;
        assert_eq!(
            lc.fetch_header(BlockId::Number(1)).await.unwrap(),
            *leaf.header()
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_header_upper_bound() {
        let client = TestClient::default();

        let db = SqliteStorage::default().await.unwrap();
        db.insert_leaf(client.leaf(2).await).await.unwrap();

        let lc = LightClient::from_genesis(db, client.clone(), client.genesis().await);
        assert_eq!(
            lc.fetch_header(BlockId::Number(1)).await.unwrap(),
            *client.leaf(1).await.header(),
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_header_invalid_proof() {
        let client = TestClient::default();
        let db = SqliteStorage::default().await.unwrap();

        // Start with an upper bound, so that `fetch_header` goes through the `client.header_proof`
        // path.
        db.insert_leaf(client.leaf(2).await).await.unwrap();

        let lc = LightClient::from_genesis(db, client.clone(), client.genesis().await);
        client.return_invalid_proof(1).await;

        let err = lc.fetch_header(BlockId::Number(1)).await.unwrap_err();
        assert!(err.to_string().contains("invalid header proof"), "{err:#}");
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_header_wrong_header() {
        let client = TestClient::default();
        let db = SqliteStorage::default().await.unwrap();

        // Start with an upper bound, so that `fetch_header` goes through the `client.header_proof`
        // path.
        db.insert_leaf(client.leaf(2).await).await.unwrap();

        let lc = LightClient::from_genesis(db, client.clone(), client.genesis().await);
        client.return_wrong_leaf(1, 0).await;

        let err = lc.fetch_header(BlockId::Number(1)).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("server returned a valid header proof for the wrong header"),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaves_in_range() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        // Fetch leaves in range [1,3) for the first time. We will need to get them from the server.
        let leaf1 = client.remember_leaf(1).await;
        let leaf2 = client.remember_leaf(2).await;
        client.remember_leaf(3).await;

        let leaves = lc.fetch_leaves_in_range(1, 3).await.unwrap();

        assert_eq!(leaves, vec![leaf1.clone(), leaf2.clone()]);

        // now remove from server and this time it should be able to fetch from local db
        client.forget_leaf(1).await;
        client.forget_leaf(2).await;
        let leaves = lc.fetch_leaves_in_range(1, 3).await.unwrap();
        assert_eq!(leaves, vec![leaf1, leaf2]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_headers_in_range() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        // Fetch headers in range [1,3) for the first time. We will need to get them from the server.
        let leaf1 = client.remember_leaf(1).await;
        let leaf2 = client.remember_leaf(2).await;
        client.remember_leaf(3).await;

        let headers = lc.fetch_headers_in_range(1, 3).await.unwrap();

        assert_eq!(
            headers,
            vec![leaf1.header().clone(), leaf2.header().clone()]
        );

        // now remove from server and this time it should be able to fetch from local db
        client.forget_leaf(1).await;
        client.forget_leaf(2).await;
        let headers = lc.fetch_headers_in_range(1, 3).await.unwrap();
        assert_eq!(
            headers,
            vec![leaf1.header().clone(), leaf2.header().clone()]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaves_in_range_invalid_proof() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );
        client.return_invalid_proof(2).await;
        let err = lc.fetch_leaves_in_range(1, 3).await.unwrap_err();
        assert!(
            err.to_string().contains(
                "server returned proof with assumption, but we have no finalized upper bound to \
                 verify assumption"
            ),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_headers_in_range_invalid_proof() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );
        client.return_invalid_proof(2).await;
        let err = lc.fetch_headers_in_range(1, 3).await.unwrap_err();
        assert!(
            err.to_string().contains(
                "server returned proof with assumption, but we have no finalized upper bound to \
                 verify assumption"
            ),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_leaves_in_range_wrong_leaf() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );
        client.return_wrong_leaf(1, 2).await;
        let err = lc.fetch_leaves_in_range(1, 4).await.unwrap_err();
        assert!(err.to_string().contains("leaf hash mismatch"), "{err:#}");
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_headers_in_range_wrong_leaf() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );
        client.return_wrong_leaf(1, 2).await;
        let err = lc.fetch_headers_in_range(1, 4).await.unwrap_err();
        assert!(err.to_string().contains("leaf hash mismatch"), "{err:#}");
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_stake_table_twice_cache() {
        let client = TestClient::default();
        let genesis = client.genesis().await;
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            genesis.clone(),
        );

        // Fetch a dynamic stake table for the first time. We will need to get it from the server.
        let epoch = genesis.first_epoch_with_dynamic_stake_table + 5;
        let expected = client.quorum_for_epoch(epoch).await.into();
        assert_eq!(*lc.quorum_for_epoch(epoch).await.unwrap(), expected);

        // Fetching the stake table again hits the cache.
        client.forget_quorum(epoch).await;
        assert_eq!(*lc.quorum_for_epoch(epoch).await.unwrap(), expected);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_stake_table_twice_storage() {
        let db = SqliteStorage::default().await.unwrap();
        let client = TestClient::default();
        let genesis = client.genesis().await;
        let lc = LightClient::from_genesis(db.clone(), client.clone(), genesis.clone());

        // Fetch a dynamic stake table for the first time. We will need to get it from the server.
        let epoch = genesis.first_epoch_with_dynamic_stake_table + 5;
        let expected = client.quorum_for_epoch(epoch).await.into();
        assert_eq!(*lc.quorum_for_epoch(epoch).await.unwrap(), expected);

        // Even if the in-memory cache is cleared, we can still get this stake table without hitting
        // the server, but fetching from the database.
        client.forget_quorum(epoch).await;
        let lc = LightClient::from_genesis(db, client.clone(), genesis);
        assert_eq!(*lc.quorum_for_epoch(epoch).await.unwrap(), expected);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_stake_table_catchup_from_lower_bound() {
        let db = SqliteStorage::default().await.unwrap();
        let client = TestClient::default();
        let genesis = client.genesis().await;
        let lc = LightClient::from_genesis(db.clone(), client.clone(), genesis.clone());

        // Fetch a stake table, causing it to be stored in the stake table, making it usable as a
        // lower bound for later catchup.
        let epoch = genesis.first_epoch_with_dynamic_stake_table + 5;
        lc.quorum_for_epoch(epoch).await.unwrap();

        // Cause the server to forget older epochs' stake table events, so that we can only catch up
        // successfully if we start from the saved lower bound.
        for epoch in 0..*epoch - 1 {
            client.forget_quorum(EpochNumber::new(epoch)).await;
        }

        // Fetch a future stake table, catching up from the saved lower bound.
        let expected = client.quorum_for_epoch(epoch + 5).await.into();
        assert_eq!(*lc.quorum_for_epoch(epoch + 5).await.unwrap(), expected);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_stake_table_cache_removal() {
        let db = SqliteStorage::default().await.unwrap();
        let client = TestClient::default();
        let genesis = client.genesis().await;
        let lc = LightClient::from_genesis_with_options(
            db.clone(),
            client.clone(),
            genesis.clone(),
            Options {
                num_stake_tables: 2,
            },
        );

        // Fetch two stake tables, causing the cache to be filled up.
        for i in 0..2 {
            let epoch = genesis.first_epoch_with_dynamic_stake_table + 5 + i;
            assert_eq!(
                *lc.quorum_for_epoch(epoch).await.unwrap(),
                client.quorum_for_epoch(epoch).await.into(),
            );
        }
        assert_eq!(lc.stake_tables.read().await.len(), 2);

        // Fetch a third stake table, causing the second-oldest stake table to be deleted.
        assert_eq!(
            *lc.quorum_for_epoch(genesis.first_epoch_with_dynamic_stake_table + 10)
                .await
                .unwrap(),
            client
                .quorum_for_epoch(genesis.first_epoch_with_dynamic_stake_table + 10)
                .await
                .into(),
        );
        assert_eq!(lc.stake_tables.read().await.len(), 2);

        // Even if the server forgets all earlier stake tables, we can still catch up because we
        // didn't remove the lower bound stake table.
        let epoch = genesis.first_epoch_with_dynamic_stake_table + 6;
        for i in 0..*epoch {
            client.forget_quorum(EpochNumber::new(i)).await;
        }
        assert_eq!(
            *lc.quorum_for_epoch(epoch).await.unwrap(),
            client.quorum_for_epoch(epoch).await.into(),
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_stake_table_invalid() {
        let db = SqliteStorage::default().await.unwrap();
        let client = TestClient::default();
        let genesis = client.genesis().await;
        let lc = LightClient::from_genesis(db.clone(), client.clone(), genesis.clone());

        let epoch = genesis.first_epoch_with_dynamic_stake_table + 5;
        client.return_invalid_quorum(epoch).await;
        let err = lc.quorum_for_epoch(epoch).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("does not match reconstructed hash"),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_payload() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        for i in 1..10 {
            let payload = client.payload(i).await;
            let res = lc.fetch_payload(BlockId::Number(i)).await.unwrap();
            assert_eq!(res.data(), &payload);
            assert_eq!(res.height(), i as u64);
            assert_eq!(res.block_hash(), client.leaf(i).await.block_hash());
            assert_eq!(res.hash(), client.leaf(i).await.payload_hash());
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_payload_invalid() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        client.return_invalid_payload(1).await;
        let err = lc.fetch_payload(BlockId::Number(1)).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("commitment of payload does not match commitment in header"),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_namespace() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        for i in 1..10 {
            let leaf = client.leaf(i).await;
            let payload = client.payload(i).await;

            for id in [
                BlockId::Number(i),
                BlockId::Hash(leaf.block_hash()),
                BlockId::PayloadHash(leaf.payload_hash()),
            ] {
                // Request a non-empty namespace.
                let ns_index = NsIndex::from(0);
                let tx = payload
                    .transaction(&TransactionIndex {
                        ns_index,
                        position: 0,
                    })
                    .unwrap();
                let txs = lc.fetch_namespace(id, tx.namespace()).await.unwrap();
                assert_eq!(txs, std::slice::from_ref(&tx));

                // Request an empty namespace.
                let txs = lc
                    .fetch_namespace(id, NamespaceId::from(u64::from(tx.namespace()) + 1))
                    .await
                    .unwrap();
                assert_eq!(txs, []);
            }
        }

        // Fetch by range.
        let ns = client
            .payload(1)
            .await
            .transaction(&TransactionIndex {
                ns_index: 0.into(),
                position: 0,
            })
            .unwrap()
            .namespace();
        let namespaces = lc.fetch_namespaces_in_range(1, 10, ns).await.unwrap();
        assert_eq!(namespaces.len(), 9);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_fetch_namespace_invalid() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );

        let payload = client.payload(1).await;
        let ns_index = NsIndex::from(0);
        let tx = payload
            .transaction(&TransactionIndex {
                ns_index,
                position: 0,
            })
            .unwrap();

        client.return_invalid_payload(1).await;
        let err = lc
            .fetch_namespace(BlockId::Number(1), tx.namespace())
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid namespace proof"),
            "{err:#}"
        );
    }
}
