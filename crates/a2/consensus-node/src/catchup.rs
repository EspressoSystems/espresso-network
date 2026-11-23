//! Catching up from API services: the node's own first, then those the operator names, then those
//! of the other validators, as their signed records name them. A node with an empty store has no
//! records yet, so it needs its own API service or one the operator names. Nothing is taken on
//! trust: a leaf chain must end in a `Certificate2` with a quorum of its epoch's stake table, and
//! state must prove against the roots of verified headers.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use bytes::Bytes;
use catchup_api::proto::{
    AccountsRequest, ChainConfigRequest, EpochRequest, HeightRequest, PayloadRequest,
    catchup_service_client::CatchupServiceClient,
};
use committable::{Commitment, Committable};
use consensus::{PayloadSource, utils::verify_new_protocol_leaf_chain};
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    BackoffParams, BlockMerkleTree, ChainConfig, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Header, Leaf2, NodeState, SeqTypes, ValidatedState,
    traits::StateCatchup,
    v0_3::{RewardAccountProofV1, RewardAccountV1, RewardAmount, RewardMerkleCommitmentV1},
    v0_4::{PermittedRewardMerkleTreeV2, RewardMerkleCommitmentV2},
    v0_7::RewardAccountV2,
};
use futures::{StreamExt, stream::FuturesUnordered};
use hotshot_types::{
    data::{VidCommitment, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
    traits::states::ValidatedState as _,
};
use jf_merkle_tree_compat::{
    AppendableMerkleTreeScheme, ForgetableMerkleTreeScheme, MerkleTreeScheme,
};
use rand::seq::SliceRandom;
use serde::de::DeserializeOwned;
use tokio::time::timeout;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use url::Url;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::store::NodeStore;

/// How long one source may take to connect and answer, the answer's body included.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// How many API services are asked at once whether they have a payload: the node's own, and
/// others drawn at random. A payload is with the store of every deployment whose node has it, so a
/// few give a hit.
const PAYLOAD_PROBES: usize = 4;

/// How long an API service may take to say whether it has a payload.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// The largest answer an API service may send; a reward tree bounds it.
const MAX_ANSWER_SIZE: usize = 256 << 20;

/// How many blocks before a height the frontier of the block tree is looked for, when no API
/// service has it at the height itself, to apply the blocks in between: the API services lag
/// behind consensus by little, and only the newest blocks are undecided.
const MAX_APPLIED_BLOCKS: u64 = 16;

type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

/// Catchup from API services.
#[derive(Clone)]
pub struct ApiCatchup {
    /// The node's own API service, asked first, and those the operator names, each with a
    /// channel that connects on first use, so that their requests share a connection.
    known: Vec<(Url, Channel)>,

    /// The consensus store, whose records name the other validators' API services.
    store: NodeStore,

    backoff: BackoffParams,
}

impl std::fmt::Debug for ApiCatchup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let urls: Vec<&str> = self.known.iter().map(|(url, _)| url.as_str()).collect();
        f.debug_struct("ApiCatchup")
            .field("known", &urls)
            .finish_non_exhaustive()
    }
}

impl ApiCatchup {
    pub fn new(own: Url, bootstrap: Vec<Url>, store: NodeStore) -> Self {
        let mut known: Vec<(Url, Channel)> = Vec::new();
        for url in std::iter::once(own).chain(bootstrap) {
            if known.iter().any(|(known, _)| *known == url) {
                continue;
            }
            match channel(&url) {
                Ok(channel) => known.push((url, channel)),
                Err(err) => tracing::warn!(%url, "cannot catch up from it: {err:#}"),
            }
        }
        Self {
            known,
            store,
            backoff: BackoffParams::default(),
        }
    }

    /// The node's own API service, those the operator names, then those the latest stored records
    /// name, with channels to them.
    fn sources(&self) -> Vec<(Url, Channel)> {
        let mut sources = self.known.clone();
        let records = match self.store.service_records(None) {
            Ok(records) => records.map(|(_, records)| records).unwrap_or_default(),
            Err(err) => {
                tracing::debug!("cannot read the service records: {err:#}");
                Vec::new()
            },
        };
        for bytes in records {
            let Ok(record) = bincode::deserialize::<SignedServiceRecord<SeqTypes>>(&bytes) else {
                continue;
            };
            let Ok(url) = record.record.api_url.parse::<Url>() else {
                continue;
            };
            if sources.iter().any(|(known, _)| *known == url) {
                continue;
            }
            match channel(&url) {
                Ok(channel) => sources.push((url, channel)),
                Err(err) => tracing::debug!(%url, "cannot catch up from it: {err:#}"),
            }
        }
        sources
    }

    /// The first answer `ask` gets from a source that checks out. A source that does not answer
    /// within [`REQUEST_TIMEOUT`] is passed over.
    async fn ask<T, F, Fut>(&self, what: &str, ask: F) -> anyhow::Result<T>
    where
        F: Fn(CatchupServiceClient<Channel>) -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        for (source, channel) in self.sources() {
            let client =
                CatchupServiceClient::new(channel).max_decoding_message_size(MAX_ANSWER_SIZE);
            let answer = timeout(REQUEST_TIMEOUT, ask(client)).await;
            match answer {
                Ok(Ok(answer)) => return Ok(answer),
                Ok(Err(err)) => tracing::debug!(%source, "{what}: {err:#}"),
                Err(_) => tracing::debug!(%source, "{what}: no answer in time"),
            }
        }
        bail!("no API service answered {what}")
    }
}

/// A channel to the API service at `url`, which connects on first use and again whenever the
/// connection fails.
impl ApiCatchup {
    /// The block tree `tree`, the tree after the block at `height`, with its frontier: from the
    /// frontier an API service has at an earlier height, with the blocks after it applied, as
    /// validating them would. The headers come from the API services for decided blocks and from
    /// the proposals this node stored for the others. Neither needs to be trusted: the result
    /// must make up `tree`'s root.
    async fn frontier_applied_forward(
        &self,
        height: u64,
        tree: &BlockMerkleTree,
    ) -> anyhow::Result<BlockMerkleTree> {
        let mut headers = self.stored_headers()?;
        for from in (height.saturating_sub(MAX_APPLIED_BLOCKS)..height).rev() {
            let frontier = self
                .ask("an earlier block frontier", |mut client| async move {
                    let answer = client
                        .get_blocks_frontier(HeightRequest { height: from })
                        .await?;
                    decode::<BlocksFrontier>(&answer.into_inner().data)
                })
                .await;
            let Ok(frontier) = frontier else {
                continue;
            };
            if (from..height).any(|h| !headers.contains_key(&h)) {
                for (h, header) in self.decided_headers(from).await.unwrap_or_default() {
                    headers.entry(h).or_default().push(header);
                }
            }
            return applied(&headers, from, &frontier, height, tree);
        }
        bail!(
            "no API service has the block frontier at any of the {MAX_APPLIED_BLOCKS} heights \
             before {height}"
        )
    }

    /// The headers of the proposals this node stored, by height, several where proposals
    /// competed for a height.
    fn stored_headers(&self) -> anyhow::Result<BTreeMap<u64, Vec<Header>>> {
        let restored = self.store.load()?;
        let mut headers: BTreeMap<u64, Vec<Header>> = BTreeMap::new();
        let stored = restored
            .proposals
            .into_values()
            .map(|proposal| proposal.data.block_header)
            .chain(
                restored
                    .anchor
                    .map(|anchor| anchor.leaf.block_header().clone()),
            );
        for header in stored {
            headers.entry(header.height()).or_default().push(header);
        }
        Ok(headers)
    }

    /// The headers of the decided blocks from `height` on, as far as an API service's leaf chain
    /// reaches.
    async fn decided_headers(&self, height: u64) -> anyhow::Result<Vec<(u64, Header)>> {
        self.ask("a leaf chain", |mut client| async move {
            let answer = client
                .get_leaf_chain(HeightRequest { height })
                .await?
                .into_inner();
            answer
                .leaves
                .iter()
                .map(|bytes| {
                    let leaf: Leaf2 = decode(bytes)?;
                    Ok((leaf.height(), leaf.block_header().clone()))
                })
                .collect()
        })
        .await
    }
}

/// `tree`, the tree after the block at `height`, with its frontier remembered: `frontier`, the
/// frontier after the block at `from`, with the commitments of the blocks from `from` up to
/// `height` appended, as validating each next block appends that of its parent. Of competing
/// headers at a height, the one whose root the tree so far makes up is taken.
fn applied(
    headers: &BTreeMap<u64, Vec<Header>>,
    from: u64,
    frontier: &BlocksFrontier,
    height: u64,
    tree: &BlockMerkleTree,
) -> anyhow::Result<BlockMerkleTree> {
    let elem = *frontier.elem().context("a frontier without its element")?;
    let mut applied = headers
        .get(&from)
        .into_iter()
        .flatten()
        .find_map(|header| {
            let mut base = ValidatedState::from_header(header).block_merkle_tree;
            base.remember(base.num_leaves() - 1, elem, frontier).ok()?;
            Some(base)
        })
        .with_context(|| format!("no header at height {from} that its frontier proves against"))?;
    for h in from..height {
        let header = headers
            .get(&h)
            .into_iter()
            .flatten()
            .find(|header| {
                ValidatedState::from_header(header)
                    .block_merkle_tree
                    .commitment()
                    == applied.commitment()
            })
            .with_context(|| format!("no header at height {h} on the chain"))?;
        applied
            .push(header.commit())
            .context("appending to the block tree")?;
    }
    ensure!(
        applied.commitment() == tree.commitment(),
        "the blocks applied do not make up the block root at height {height}"
    );
    Ok(applied)
}

/// `tree` with the frontier an API service has after the block at `height`.
async fn remembered(
    mut client: CatchupServiceClient<Channel>,
    height: u64,
    mut tree: BlockMerkleTree,
) -> anyhow::Result<BlockMerkleTree> {
    let answer = client
        .get_blocks_frontier(HeightRequest { height })
        .await?
        .into_inner();
    let frontier: BlocksFrontier = decode(&answer.data)?;
    let elem = frontier.elem().context("a frontier without its element")?;
    tree.remember(tree.num_leaves() - 1, *elem, &frontier)
        .context("the frontier does not prove against the block root")?;
    Ok(tree)
}

/// Payloads from the API services, in the order state comes from them: the node's own store,
/// which acknowledged every payload this node voted vote2 on, first.
#[async_trait]
impl PayloadSource for ApiCatchup {
    /// Asks the first source, the node's own API service unless it cannot be reached, and others
    /// drawn at random, whether they have the payload, all at once, and downloads it from the
    /// first that says so. Each time the fetcher asks again, others are drawn.
    async fn payload(&self, commitment: VidCommitment2) -> anyhow::Result<Option<Bytes>> {
        let commitment = VidCommitment::V2(commitment).to_string();
        let mut probed = self.sources();
        let mut others = probed.split_off(probed.len().min(1));
        others.shuffle(&mut rand::thread_rng());
        probed.extend(others.into_iter().take(PAYLOAD_PROBES - 1));

        let mut probes: FuturesUnordered<_> = probed
            .into_iter()
            .map(|(url, channel)| {
                let request = PayloadRequest {
                    commitment: commitment.clone(),
                };
                async move {
                    let mut client = CatchupServiceClient::new(channel.clone());
                    let has = timeout(PROBE_TIMEOUT, client.has_payload(request)).await;
                    (url, channel, has)
                }
            })
            .collect();
        while let Some((url, channel, has)) = probes.next().await {
            match has {
                Ok(Ok(answer)) if answer.get_ref().has => {},
                Ok(Ok(_)) => continue,
                Ok(Err(err)) => {
                    tracing::debug!(%url, "has the payload {commitment}: {err:#}");
                    continue;
                },
                Err(_) => {
                    tracing::debug!(%url, "has the payload {commitment}: no answer in time");
                    continue;
                },
            }
            let mut client =
                CatchupServiceClient::new(channel).max_decoding_message_size(MAX_ANSWER_SIZE);
            let request = PayloadRequest {
                commitment: commitment.clone(),
            };
            match timeout(REQUEST_TIMEOUT, client.get_payload(request)).await {
                Ok(Ok(answer)) => return Ok(Some(answer.into_inner().data)),
                Ok(Err(err)) => tracing::debug!(%url, "the payload {commitment}: {err:#}"),
                Err(_) => tracing::debug!(%url, "the payload {commitment}: no answer in time"),
            }
        }
        Ok(None)
    }
}

fn channel(url: &Url) -> anyhow::Result<Channel> {
    let mut endpoint = Endpoint::from_shared(url.to_string())?
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        // A connection is shared by many requests; pings drop one that died without closing.
        .http2_keep_alive_interval(Duration::from_secs(10))
        .keep_alive_timeout(Duration::from_secs(20))
        .keep_alive_while_idle(true);
    if url.scheme() == "https" {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    Ok(endpoint.connect_lazy())
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
    bincode::deserialize(bytes).context("decoding the answer")
}

#[async_trait]
impl StateCatchup for ApiCatchup {
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        self.ask("a leaf chain", |mut client| {
            let coordinator = coordinator.clone();
            async move {
                let answer = client
                    .get_leaf_chain(HeightRequest { height })
                    .await?
                    .into_inner();
                let leaves = answer
                    .leaves
                    .iter()
                    .map(|bytes| decode::<Leaf2>(bytes))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                let cert2: Certificate2<SeqTypes> = decode(&answer.cert2)?;
                let lock = UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));
                verify_new_protocol_leaf_chain(leaves, &coordinator, height, &lock, cert2).await
            }
        })
        .await
    }

    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        _instance: &NodeState,
        height: u64,
        _view: ViewNumber,
        root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        let encoded = accounts
            .iter()
            .map(|account| bincode::serialize(account).map(Bytes::from))
            .collect::<Result<Vec<_>, _>>()?;
        self.ask("fee accounts", |mut client| {
            let encoded = encoded.clone();
            async move {
                let answer = client
                    .get_fee_accounts(AccountsRequest {
                        height,
                        accounts: encoded,
                    })
                    .await?
                    .into_inner();
                let proofs: Vec<FeeAccountProof> = decode(&answer.data)?;
                ensure!(proofs.len() == accounts.len(), "a proof per account");
                for (proof, account) in proofs.iter().zip(accounts) {
                    ensure!(proof.account == account.0, "a proof of another account");
                    proof
                        .verify(&root)
                        .with_context(|| format!("the proof of fee account {account}"))?;
                }
                Ok(proofs)
            }
        })
        .await
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        _retry: usize,
        _instance: &NodeState,
        height: u64,
        _view: ViewNumber,
        tree: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        let template = tree.clone();
        let asked = self
            .ask("the block frontier", |client| {
                remembered(client, height, template.clone())
            })
            .await;
        *tree = match asked {
            Ok(remembered) => remembered,
            // No API service has the state after the block yet, as for a block not decided.
            Err(err) => self
                .frontier_applied_forward(height, &template)
                .await
                .with_context(|| format!("{err:#}"))?,
        };
        Ok(())
    }

    async fn try_fetch_chain_config(
        &self,
        _retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.ask("a chain config", |mut client| async move {
            let answer = client
                .get_chain_config(ChainConfigRequest {
                    commitment: commitment.to_string(),
                })
                .await?
                .into_inner();
            let config: ChainConfig = decode(&answer.data)?;
            ensure!(
                config.commit() == commitment,
                "a chain config of another commitment"
            );
            Ok(config)
        })
        .await
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        _retry: usize,
        height: u64,
        _view: ViewNumber,
        root: RewardMerkleCommitmentV2,
        _accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        // The whole tree, rebuilt from every balance, so no account is missing from it.
        self.ask("the reward tree", |mut client| async move {
            let answer = client
                .get_reward_balances_v2(HeightRequest { height })
                .await?
                .into_inner();
            let balances: Vec<(RewardAccountV2, RewardAmount)> = decode(&answer.data)?;
            let tree = PermittedRewardMerkleTreeV2::try_from_kv_set(balances).await?;
            ensure!(
                tree.tree.commitment() == root,
                "a reward tree of another root"
            );
            Ok(tree)
        })
        .await
    }

    /// The v1 reward tree belongs to blocks before the new protocol, which no a2 chain has.
    async fn try_fetch_reward_accounts_v1(
        &self,
        _retry: usize,
        _instance: &NodeState,
        height: u64,
        _view: ViewNumber,
        _root: RewardMerkleCommitmentV1,
        _accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        bail!("API services keep no v1 reward accounts, asked at height {height}")
    }

    /// Returned as the API service sends it; whoever asks checks it against the stake table.
    async fn try_fetch_state_cert(
        &self,
        _retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.ask("a state certificate", |mut client| async move {
            let answer = client
                .get_state_cert(EpochRequest { epoch })
                .await?
                .into_inner();
            decode(&answer.data)
        })
        .await
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        "API services".into()
    }

    fn is_local(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use espresso_types::NodeState;
    use jf_merkle_tree_compat::LookupResult;
    use versions::NEW_PROTOCOL_VERSION;

    use super::*;

    /// A chain from genesis through height 3, each block with the state after it, the block
    /// tree root in each header made up by the commitments of the blocks before it.
    async fn chain() -> Vec<(Leaf2, ValidatedState)> {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let mut chain = vec![(
            Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            state,
        )];
        for _ in 0..3 {
            let (parent, state) = chain.last().unwrap().clone();
            let mut next = state.clone();
            next.block_merkle_tree
                .push(parent.block_header().commit())
                .unwrap();
            let mut leaf = parent.clone();
            let header = leaf.block_header_mut();
            *header.height_mut() += 1;
            *header.block_merkle_tree_root_mut() = next.block_merkle_tree.commitment();
            chain.push((leaf, next));
        }
        chain
    }

    fn frontier(state: &ValidatedState) -> BlocksFrontier {
        let tree = &state.block_merkle_tree;
        match tree.lookup(tree.num_leaves() - 1) {
            LookupResult::Ok(_, proof) => proof,
            _ => panic!("the last leaf is in memory"),
        }
    }

    /// The frontier at an earlier height, with the blocks in between applied, makes up the
    /// tree at the height asked for, its frontier remembered. A competing header at a height is
    /// passed over, and a root the blocks do not make up is refused.
    #[tokio::test]
    async fn applies_blocks_to_an_earlier_frontier() {
        let chain = chain().await;
        let mut headers: BTreeMap<u64, Vec<Header>> = BTreeMap::new();
        for (leaf, _) in &chain {
            headers
                .entry(leaf.height())
                .or_default()
                .push(leaf.block_header().clone());
        }
        let mut competing = chain[2].0.block_header().clone();
        *competing.block_merkle_tree_root_mut() = chain[1].1.block_merkle_tree.commitment();
        headers.get_mut(&2).unwrap().insert(0, competing);
        let target = ValidatedState::from_header(chain[3].0.block_header()).block_merkle_tree;

        let tree = applied(&headers, 1, &frontier(&chain[1].1), 3, &target).unwrap();
        assert_eq!(tree.commitment(), target.commitment());
        assert!(
            matches!(tree.lookup(tree.num_leaves() - 1), LookupResult::Ok(..)),
            "the frontier is remembered"
        );

        let other = ValidatedState::from_header(chain[2].0.block_header()).block_merkle_tree;
        applied(&headers, 1, &frontier(&chain[1].1), 3, &other)
            .expect_err("the blocks do not make up another root");
    }

    /// A catchup service over a store holding `payloads`, keyed by commitment, at the URL returned.
    async fn catchup_service(payloads: &[(VidCommitment2, &'static [u8])]) -> Url {
        let objects = chain_store::payloads::PayloadStore::new(
            Arc::new(object_store::memory::InMemory::new()),
            "p",
            None,
        );
        let store = chain_store::ChainStore::new(chain_store::mem::MemBackend::default())
            .with_payloads(objects);
        for (commitment, payload) in payloads {
            let commitment = VidCommitment::V2(*commitment).to_string();
            store
                .put_payload(&commitment, Bytes::from_static(payload), None)
                .await
                .unwrap();
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let catchup = api_service::catchup::Catchup::new(store, 10);
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(
                    catchup_api::proto::catchup_service_server::CatchupServiceServer::new(catchup),
                )
                .serve_with_incoming(tonic::transport::server::TcpIncoming::from(listener)),
        );
        url
    }

    /// A payload is downloaded from a service that has it, whichever is asked first, and a
    /// payload no service has is not found.
    #[tokio::test]
    async fn downloads_a_payload_from_a_service_that_has_it() {
        let held = VidCommitment2::default();
        let own = catchup_service(&[]).await;
        let other = catchup_service(&[(held, b"payload")]).await;
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = crate::store::open(dir.path().into()).await.unwrap();
        let catchup = ApiCatchup::new(own, vec![other], store);

        let payload = catchup.payload(held).await.unwrap();
        assert_eq!(payload.as_deref(), Some(&b"payload"[..]));

        let empty = catchup_service(&[]).await;
        let catchup = ApiCatchup::new(empty, Vec::new(), catchup.store.clone());
        assert_eq!(catchup.payload(held).await.unwrap(), None);
    }
}
