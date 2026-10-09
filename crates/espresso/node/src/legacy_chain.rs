//! Chains recorded from legacy consensus (before 0.6), replayed into a query node that runs no
//! consensus.
//!
//! Mainnet and decaf still serve data from before 0.6: old headers, VID schemes, reward trees,
//! state certificates and the stake table contract's V1 and V2 event history. Once legacy
//! consensus is gone no test network can produce that data, so `record_legacy_chains` recorded
//! it while it still could, and [`LegacyChain::replay`] serves it again. A replayed node is a
//! follower fed the recorded blocks instead of a light client, on an L1 restored from the
//! recording, so the stake table, rewards and merklized state are derived by the same code that
//! derives them on a live node.
//!
//! The recordings cannot be regenerated once legacy consensus is deleted. A change to how any
//! recorded type deserializes has to keep reading them, as the `data/v*` reference vectors do.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
};
use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use espresso_types::{
    Header, SeqTypes,
    traits::{PersistenceOptions, SequencerPersistence},
};
use futures::FutureExt;
use hotshot_query_service::{
    availability::{BlockQueryData, LeafQueryData, VidCommonQueryData},
    data_source::storage::sql::testing::TmpDb,
    types::HeightIndexed,
};
use hotshot_types::{
    addr::NetAddr, network::NetworkConfig, simple_certificate::LightClientStateUpdateCertificateV2,
    x25519,
};
use http_client::{Client, error::ClientErr};
use serde::{Deserialize, Serialize};
use test_utils::reserve_tcp_port;
use tokio::time::sleep;

use crate::{
    CatchupParams, Genesis, L1Params, SequencerApiVersion,
    api::{
        Options,
        data_source::testing::TestableSequencerDataSource,
        options::Query,
        sql::{DataSource as SqlDataSource, impl_testable_data_source::tmp_options},
    },
    follower::{
        ChainSource, FollowerContext, FollowerOptions, FollowerParams, VerifiedBlock,
        init_replay_node,
    },
    persistence,
};

/// A chain recorded from legacy consensus, as stored under `data/legacy-chains/<name>`.
pub struct LegacyChain {
    pub name: String,
    pub genesis: Genesis,
    pub network_config: NetworkConfig<SeqTypes>,
    pub blocks: Vec<RecordedBlock>,
    /// Every epoch's light client state certificate the recorded network produced.
    pub state_certs: Vec<(u64, LightClientStateUpdateCertificateV2<SeqTypes>)>,
    /// The network config registered on the stake table's V3 contract, if the recording did.
    pub network_config_update: Option<RecordedNetworkConfigUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordedBlock {
    pub leaf: LeafQueryData<SeqTypes>,
    pub block: BlockQueryData<SeqTypes>,
    pub vid_common: VidCommonQueryData<SeqTypes>,
}

/// What `update_network_config` registered for a validator, for tests to look for in the stake
/// table the replayed node derives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedNetworkConfigUpdate {
    pub validator: Address,
    pub x25519_key: x25519::PublicKey,
    pub p2p_addr: NetAddr,
}

/// A query node serving a replayed [`LegacyChain`].
pub struct ReplayNode {
    pub node: FollowerContext<persistence::sql::Persistence>,
    pub client: Client<ClientErr, SequencerApiVersion>,
    pub url: url::Url,
    /// The recorded L1, which the node reads the stake table and rewards from.
    pub l1: AnvilInstance,
    storage: TmpDb,
}

impl LegacyChain {
    pub fn load(name: &str) -> anyhow::Result<Self> {
        let dir = chain_dir(name);
        let read = |file: &str| {
            fs::read_to_string(dir.join(file)).with_context(|| format!("reading {name}/{file}"))
        };
        let network_config_update = if dir.join(NETWORK_CONFIG_UPDATE).exists() {
            Some(serde_json::from_str(&read(NETWORK_CONFIG_UPDATE)?)?)
        } else {
            None
        };
        Ok(Self {
            name: name.into(),
            genesis: Genesis::from_file(dir.join(GENESIS))?,
            network_config: serde_json::from_str(&read(NETWORK_CONFIG)?)?,
            blocks: serde_json::from_str(&read(BLOCKS)?)?,
            state_certs: serde_json::from_str(&read(STATE_CERTS)?)?,
            network_config_update,
        })
    }

    /// The height of the last recorded block.
    pub fn tip(&self) -> u64 {
        self.blocks
            .last()
            .expect("a recorded chain has blocks")
            .leaf
            .height()
    }

    /// Starts a query node on this chain and returns once it has stored every block and derived
    /// the merklized state up to the tip.
    pub async fn replay(&self) -> anyhow::Result<ReplayNode> {
        let l1 = Anvil::new()
            .args(["--slots-in-an-epoch", "0"])
            .arg("--load-state")
            .arg(chain_dir(&self.name).join(L1_STATE))
            .spawn();

        let storage = SqlDataSource::create_storage().await;
        let mut db = tmp_options(&storage);
        let persistence = db.create().await?;
        // There are no peers to fetch either from.
        persistence.save_config(&self.network_config).await?;
        for (epoch, cert) in &self.state_certs {
            persistence.insert_state_cert(*epoch, cert.clone()).await?;
        }

        let genesis = self.genesis.clone();
        let l1_params = L1Params {
            urls: vec![l1.endpoint_url()],
            options: Default::default(),
        };
        let source = Arc::new(ReplaySource {
            blocks: self.blocks.clone(),
        });
        let port = reserve_tcp_port().context("no free port for the replayed query API")?;
        let url: url::Url = format!("http://localhost:{port}").parse()?;
        let params = replay_params(url.clone());
        let handle = Options::with_port(port)
            .query_sql(Query::default(), db)
            .catchup(Default::default())
            .config(Default::default())
            .explorer(Default::default())
            .light_client(Default::default())
            .serve(move |metrics, sink, _| {
                async move {
                    init_replay_node(
                        genesis,
                        params,
                        metrics,
                        persistence,
                        l1_params,
                        sink,
                        source,
                    )
                    .await
                }
                .boxed()
            })
            .await
            .context("replayed node should start")?;
        let node = FollowerContext::new(handle);

        let client = Client::new(url.clone());
        ensure!(
            client.connect(Some(Duration::from_secs(60))).await,
            "replayed query API did not come up"
        );
        wait_for_replay(&client, self.tip()).await?;
        Ok(ReplayNode {
            node,
            client,
            url,
            l1,
            storage,
        })
    }
}

impl ReplayNode {
    /// Another connection to the node's consensus storage.
    pub async fn persistence(&self) -> anyhow::Result<persistence::sql::Persistence> {
        tmp_options(&self.storage).create().await
    }
}

const GENESIS: &str = "genesis.toml";
const NETWORK_CONFIG: &str = "network-config.json";
const BLOCKS: &str = "blocks.json";
const STATE_CERTS: &str = "state-certs.json";
const NETWORK_CONFIG_UPDATE: &str = "network-config-update.json";
const L1_STATE: &str = "l1-state.json";

fn chain_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/legacy-chains")
        .join(name)
}

/// The node is its own only peer: it serves catchup from the state it derived itself.
fn replay_params(own_url: url::Url) -> FollowerParams {
    FollowerParams {
        upstreams: vec![],
        config_peers: None,
        catchup: CatchupParams {
            state_peers: vec![own_url],
            backoff: Default::default(),
            base_timeout: Duration::from_secs(2),
            local_timeout: Duration::from_secs(5),
        },
        bootstrap_epoch_catchup_timeout: Duration::from_secs(30),
        options: FollowerOptions {
            poll_interval: Duration::from_millis(100),
            // Skipping ahead would leave the gap to a backfill with nothing to fetch from.
            max_blocks_per_poll: u64::MAX,
            light_client: Default::default(),
            light_client_db: Default::default(),
            light_client_genesis: None,
        },
    }
}

/// Until the node has stored every block and its merklized state has caught up with them.
///
/// The stages stall separately: the follower on a block it cannot store, the state loop on the
/// first header whose derived roots do not match, which it logs as an error and retries.
async fn wait_for_replay(
    client: &Client<ClientErr, SequencerApiVersion>,
    tip: u64,
) -> anyhow::Result<()> {
    wait_for_height(
        client,
        "node/block-height",
        tip + 1,
        Duration::from_secs(120),
    )
    .await
    .context("storing the recorded blocks stalled")?;
    wait_for_height(
        client,
        "block-state/block-height",
        tip,
        Duration::from_secs(300),
    )
    .await
    .context("rebuilding the merklized state stalled; the state loop logs which root differs")
}

async fn wait_for_height(
    client: &Client<ClientErr, SequencerApiVersion>,
    route: &str,
    target: u64,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut reached = None;
    while Instant::now() < deadline {
        if let Ok(height) = client.get::<u64>(route).send().await {
            if height >= target {
                return Ok(());
            }
            reached = Some(height);
        }
        sleep(Duration::from_millis(200)).await;
    }
    bail!("{route} reached {reached:?} of {target} in {timeout:?}")
}

/// Hands a follower the recorded blocks, as a light client hands it verified ones.
struct ReplaySource {
    blocks: Vec<RecordedBlock>,
}

#[async_trait]
impl ChainSource for ReplaySource {
    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(self.blocks.len() as u64)
    }

    async fn fetch_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<VerifiedBlock>> {
        let blocks = self
            .blocks
            .get(from as usize..to as usize)
            .with_context(|| format!("blocks {from}..{to} were not recorded"))?;
        Ok(blocks
            .iter()
            .cloned()
            .map(
                |RecordedBlock {
                     leaf,
                     block,
                     vid_common,
                 }| VerifiedBlock {
                    leaf,
                    block,
                    vid_common,
                },
            )
            .collect())
    }

    /// No legacy block has one.
    async fn fetch_cert2(
        &self,
        _header: &Header,
    ) -> anyhow::Result<Option<espresso_types::Certificate2<SeqTypes>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod record;
