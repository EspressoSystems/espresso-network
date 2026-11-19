//! Catching up from API services: the node's own first, then those the operator names, then those
//! of the other validators, as their signed records name them. A node with an empty store has no
//! records yet, so it needs its own API service or one the operator names. Nothing is taken on trust: a leaf chain must end in a
//! `Certificate2` with a quorum of its epoch's stake table, and state must prove against the roots
//! of verified headers.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use consensus::{
    discovery::SignedServiceRecord, store::ConsensusStore, utils::verify_new_protocol_leaf_chain,
};
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
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use url::Url;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::proto::{
    AccountsRequest, ChainConfigRequest, EpochRequest, HeightRequest,
    catchup_service_client::CatchupServiceClient,
};

/// How long one request to an API service may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// The largest answer an API service may send; a reward tree bounds it.
const MAX_ANSWER_SIZE: usize = 256 << 20;

type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

/// Catchup from API services.
#[derive(Clone)]
pub struct ApiCatchup {
    /// The node's own API service, asked first, and those the operator names.
    own: Url,
    bootstrap: Vec<Url>,

    /// The consensus store, whose records name the other validators' API services.
    store: ConsensusStore<SeqTypes>,

    backoff: BackoffParams,
}

impl std::fmt::Debug for ApiCatchup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiCatchup")
            .field("own", &self.own.as_str())
            .finish_non_exhaustive()
    }
}

impl ApiCatchup {
    pub fn new(own: Url, bootstrap: Vec<Url>, store: ConsensusStore<SeqTypes>) -> Self {
        Self {
            own,
            bootstrap,
            store,
            backoff: BackoffParams::default(),
        }
    }

    /// The node's own API service, those the operator names, then those the latest stored records
    /// name.
    fn sources(&self) -> Vec<Url> {
        let mut sources = vec![self.own.clone()];
        for url in &self.bootstrap {
            if !sources.contains(url) {
                sources.push(url.clone());
            }
        }
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
            match record.record.api_url.parse::<Url>() {
                Ok(url) if !sources.contains(&url) => sources.push(url),
                _ => {},
            }
        }
        sources
    }

    /// The first answer `ask` gets from a source that checks out.
    async fn ask<T, F, Fut>(&self, what: &str, ask: F) -> anyhow::Result<T>
    where
        F: Fn(CatchupServiceClient<Channel>) -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        for source in self.sources() {
            let client = match connect(&source).await {
                Ok(client) => client,
                Err(err) => {
                    tracing::debug!(%source, "cannot connect: {err:#}");
                    continue;
                },
            };
            match ask(client).await {
                Ok(answer) => return Ok(answer),
                Err(err) => tracing::debug!(%source, "{what}: {err:#}"),
            }
        }
        bail!("no API service answered {what}")
    }
}

async fn connect(url: &Url) -> anyhow::Result<CatchupServiceClient<Channel>> {
    let mut endpoint = Endpoint::from_shared(url.to_string())?
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT);
    if url.scheme() == "https" {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    Ok(CatchupServiceClient::new(endpoint.connect().await?)
        .max_decoding_message_size(MAX_ANSWER_SIZE))
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
