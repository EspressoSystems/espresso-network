//! Catching up from API services: the node's own first, then those the operator names, then those
//! of the other validators, as their signed records name them. A node with an empty store has no
//! records yet, so it needs its own API service or one the operator names. Nothing is taken on
//! trust: a leaf chain must end in a `Certificate2` with a quorum of its epoch's stake table, and
//! state must prove against the roots of verified headers.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use catchup_api::proto::{
    AccountsRequest, ChainConfigRequest, EpochRequest, HeightRequest,
    catchup_service_client::CatchupServiceClient,
};
use committable::{Commitment, Committable};
use consensus::utils::verify_new_protocol_leaf_chain;
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    BackoffParams, BlockMerkleTree, ChainConfig, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Leaf2, NodeState, SeqTypes,
    traits::StateCatchup,
    v0_3::{RewardAccountProofV1, RewardAccountV1, RewardAmount, RewardMerkleCommitmentV1},
    v0_4::{PermittedRewardMerkleTreeV2, RewardMerkleCommitmentV2},
    v0_7::RewardAccountV2,
};
use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
};
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
use serde::de::DeserializeOwned;
use tokio::time::timeout;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use url::Url;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::store::NodeStore;

/// How long one source may take to connect and answer, the answer's body included.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// The largest answer an API service may send; a reward tree bounds it.
const MAX_ANSWER_SIZE: usize = 256 << 20;

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
            .map(bincode::serialize)
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
        *tree = self
            .ask("the block frontier", |mut client| {
                let mut tree = template.clone();
                async move {
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
            })
            .await?;
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
