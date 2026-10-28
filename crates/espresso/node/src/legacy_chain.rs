//! Chains recorded from legacy consensus (before 0.6), replayed through a query node that runs
//! no consensus: a follower fed the recorded blocks, on an anvil that mines the recorded L1
//! transactions again, so the stake table, rewards and merklized state are derived by the code a
//! live node runs. Mainnet and decaf still serve pre-0.6 data, and once legacy consensus is gone
//! nothing else can produce it; `data/legacy-chains/README.md` describes the chains.
//!
//! The recordings cannot be regenerated once legacy consensus is deleted; the recorder is kept on
//! the `legacy-chain-recorder` branch. A change to how any recorded type deserializes has to keep
//! reading them, as the `data/v*` reference vectors do.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, Bytes},
    providers::{Provider, ProviderBuilder, ext::AnvilApi},
    rpc::types::Block,
};
use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use espresso_types::{
    Certificate2, Header, SeqTypes,
    traits::{PersistenceOptions, SequencerPersistence, StateCertPersistence},
};
use futures::FutureExt;
use hotshot_query_service::{
    availability::{BlockQueryData, LeafQueryData, VidCommonQueryData},
    types::HeightIndexed,
};
use hotshot_types::{
    addr::NetAddr, network::NetworkConfig, simple_certificate::LightClientStateUpdateCertificateV2,
    utils::epoch_from_block_number, x25519,
};
use http_client::{Client, error::ClientErr};
use serde::{Deserialize, Serialize};
use test_utils::reserve_tcp_port;
use tokio::time::sleep;

use crate::{
    CatchupParams, Genesis, L1Params, SequencerApiVersion,
    api::{Options, data_source::testing::TestableSqlDataSource, options::Query},
    follower::{
        ChainSource, FollowerContext, FollowerOptions, FollowerParams, VerifiedBlock,
        init_replay_node,
    },
};

/// A chain recorded from legacy consensus, as stored under `data/legacy-chains/<name>`.
pub struct LegacyChain {
    pub name: String,
    pub genesis: Genesis,
    pub network_config: NetworkConfig<SeqTypes>,
    pub blocks: Vec<RecordedBlock>,
    /// One per finished epoch, keyed by epoch.
    pub state_certs: Vec<(u64, LightClientStateUpdateCertificateV2<SeqTypes>)>,
    /// The network config registered on the stake table's V3 contract, if the recording did.
    pub network_config_update: Option<RecordedNetworkConfigUpdate>,
    /// Indexed by block number; block 0 is anvil's genesis and has no transactions.
    pub l1_blocks: Vec<RecordedL1Block>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordedBlock {
    pub leaf: LeafQueryData<SeqTypes>,
    pub block: BlockQueryData<SeqTypes>,
    pub vid_common: VidCommonQueryData<SeqTypes>,
}

/// An L1 block as its timestamp and signed transactions, which is all a replay needs to mine it
/// again: contract addresses, logs and block numbers depend only on the transactions and their
/// order. Block hashes do not survive a replay, and nothing the node runs compares them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedL1Block {
    pub timestamp: u64,
    /// In the order mined, as `eth_sendRawTransaction` takes them.
    pub transactions: Vec<Bytes>,
}

/// What `update_network_config` registered, for tests to find in the derived stake table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedNetworkConfigUpdate {
    pub validator: Address,
    pub x25519_key: x25519::PublicKey,
    pub p2p_addr: NetAddr,
}

/// A query node serving a replayed [`LegacyChain`], on the SQL backend of `D`.
pub struct ReplayNode<D: TestableSqlDataSource> {
    pub node: FollowerContext<<D::Options as PersistenceOptions>::Persistence>,
    pub client: Client<ClientErr, SequencerApiVersion>,
    pub url: url::Url,
    pub l1: AnvilInstance,
    storage: D::Storage,
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
            l1_blocks: serde_json::from_str(&read(L1_BLOCKS)?)?,
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

    pub fn epoch_height(&self) -> u64 {
        self.genesis
            .epoch_height
            .expect("a recorded chain has epochs")
    }

    /// The epoch of the last recorded block.
    pub fn last_epoch(&self) -> u64 {
        epoch_from_block_number(self.tip(), self.epoch_height())
    }

    /// Starts a query node on this chain and returns once it has stored every block and derived
    /// the merklized state up to the tip.
    pub async fn replay<D: TestableSqlDataSource>(&self) -> anyhow::Result<ReplayNode<D>> {
        let l1 = replay_l1(&self.l1_blocks)
            .await
            .with_context(|| format!("replaying the {} chain's L1", self.name))?;

        let storage = D::create_storage().await;
        let persistence = D::persistence_options(&storage).create().await?;
        // There are no peers to fetch the config or state certs from.
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
            .query_sql(Query::default(), D::tmp_options(&storage))
            .catchup(Default::default())
            .config(Default::default())
            .explorer(Default::default())
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

impl<D: TestableSqlDataSource> ReplayNode<D> {
    /// Another connection to the node's consensus storage.
    pub async fn persistence(
        &self,
    ) -> anyhow::Result<<D::Options as PersistenceOptions>::Persistence> {
        D::persistence_options(&self.storage).create().await
    }
}

const GENESIS: &str = "genesis.toml";
const NETWORK_CONFIG: &str = "network-config.json";
const BLOCKS: &str = "blocks.json";
const STATE_CERTS: &str = "state-certs.json";
const NETWORK_CONFIG_UPDATE: &str = "network-config-update.json";
const L1_BLOCKS: &str = "l1-blocks.json";

fn chain_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/legacy-chains")
        .join(name)
}

/// Starts an anvil at the recorded genesis timestamp and mines the recorded blocks one by one.
async fn replay_l1(blocks: &[RecordedL1Block]) -> anyhow::Result<AnvilInstance> {
    let (genesis, rest) = blocks
        .split_first()
        .context("the recorded L1 has a genesis block")?;
    // As the recorder ran anvil: every block is finalized at once, and the default accounts hold
    // enough to fund the recorded transactions.
    let anvil = Anvil::new()
        .args([
            "--slots-in-an-epoch",
            "0",
            "--balance",
            "1000000",
            "--no-mining",
        ])
        .args(["--timestamp", &genesis.timestamp.to_string()])
        .spawn();
    let provider = ProviderBuilder::new().connect_http(anvil.endpoint_url());
    let mined = l1_block(&provider, 0).await?;
    ensure!(
        mined.header.timestamp == genesis.timestamp,
        "anvil's genesis block has timestamp {} instead of the recorded {}",
        mined.header.timestamp,
        genesis.timestamp
    );
    for (number, block) in (1u64..).zip(rest) {
        mine_l1_block(&provider, number, block).await?;
    }
    Ok(anvil)
}

async fn mine_l1_block(
    provider: &impl Provider,
    number: u64,
    block: &RecordedL1Block,
) -> anyhow::Result<()> {
    provider
        .anvil_set_next_block_timestamp(block.timestamp)
        .await
        .with_context(|| format!("setting the timestamp of L1 block {number}"))?;
    let mut sent = vec![];
    for (i, tx) in block.transactions.iter().enumerate() {
        // Not awaited: with mining off a pending transaction never confirms.
        let pending = provider
            .send_raw_transaction(tx)
            .await
            .with_context(|| format!("sending transaction {i} of L1 block {number}"))?;
        sent.push(*pending.tx_hash());
    }
    provider.anvil_mine(Some(1), None).await?;
    let mined = l1_block(provider, number).await?;
    ensure!(
        mined.header.timestamp == block.timestamp,
        "L1 block {number} was mined at {} instead of the recorded {}",
        mined.header.timestamp,
        block.timestamp
    );
    let hashes: Vec<_> = mined.transactions.hashes().collect();
    ensure!(
        hashes == sent,
        "L1 block {number} holds transactions {hashes:?} instead of the recorded {sent:?}"
    );
    Ok(())
}

async fn l1_block(provider: &impl Provider, number: u64) -> anyhow::Result<Block> {
    provider
        .get_block_by_number(number.into())
        .await?
        .with_context(|| format!("anvil has no L1 block {number}"))
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
            http_delay: Duration::ZERO,
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
    ) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        Ok(None)
    }
}
