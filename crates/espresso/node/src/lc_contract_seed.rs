//! Seeds the post-start epoch catchup from the LC contract.

use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow, bail, ensure};
use committable::Committable;
use espresso_types::{
    EpochRootInstallError, Header, LcContractVotingStakeTable, Leaf2, NodeState, SeqTypes,
    traits::SequencerPersistence,
};
use futures::{FutureExt, future::try_join_all};
use hotshot_new_protocol::{storage::NewProtocolStorage, utils::CertifiedLeafChain};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::{EpochMembershipCoordinator, LcContractSeedFn, LcContractSeedOutcome},
    message::UpgradeLock,
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, MembershipSnapshot},
        node_implementation::NodeImplementation,
    },
    utils::{is_epoch_root, root_block_in_epoch},
};
use light_client::consensus::header::HeaderProof;
use thiserror::Error;
use tokio::time::timeout;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// One attempt to fetch and verify the anchor chain; a forged leaf in it makes every `Cert2`
/// answer fail, so the leaves are fetched again after this.
const ANCHOR_TRY_TIMEOUT: Duration = Duration::from_secs(6);

/// The LC-contract seed of the catchup, behind `LcContractSeedFn`.
///
/// - Trust: `ST(E')` keys and stake come from the SNARK-verified `votingStakeTableState`. The seed
///   header is used only as the candidate quorum for the anchor until it is proven. Every installed
///   table comes from a header proven against the anchor `H*(E'-1)+1` or reached from it by parent
///   links.
/// - Proven headers: `root(E'-5) ..= root(E'-2)`, those above the held pair. Chain-linked:
///   `root(E'-1)`, `H*(E'-1)-3` (`DRB(E')`). Installed: the epochs above the held pair within
///   `ST(E'-3) ..= ST(E'+1)`, and `DRB(E')`; rewards of `E' ..= E'+2` are then computable.
/// - Rejections: seed chain config must equal the known config; a target below `E'` fails fast; a
///   hash mismatch evicts the epochs this seed installed and errors, never panics. Cert checks use
///   the restored upgrade lock.
/// - Non-answering peers are DEBUG only; one WARN per failed attempt (by the coordinator).
pub(crate) struct LcContractSeed<I: NodeImplementation<SeqTypes>, P: SequencerPersistence>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    node_state: NodeState,
    upgrade_lock: UpgradeLock<SeqTypes>,
    stake_table_capacity: usize,
    /// Set once the request-response protocol exists, which needs the consensus handle built
    /// after the hook is installed. An attempt before that fails and is retried.
    requests: RequestsCell<I, P>,
}

/// Late-bound request-response protocol of a seed.
pub(crate) type RequestsCell<I, P> = Arc<OnceLock<RequestResponseProtocol<I, P>>>;

impl<I: NodeImplementation<SeqTypes>, P: SequencerPersistence> LcContractSeed<I, P>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    /// A seed over `node_state` (L1, LC contract), the node's restored `upgrade_lock` and the
    /// `stake_table_capacity` of the LC contract commitment.
    pub fn new(
        node_state: NodeState,
        upgrade_lock: UpgradeLock<SeqTypes>,
        stake_table_capacity: usize,
    ) -> Self {
        Self {
            node_state,
            upgrade_lock,
            stake_table_capacity,
            requests: Arc::default(),
        }
    }

    /// The cell to set with the request-response protocol once it exists.
    pub fn requests_cell(&self) -> RequestsCell<I, P> {
        Arc::clone(&self.requests)
    }

    /// One seed attempt for `catchup(target)` with held pair top `start`.
    pub async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<LcContractSeedOutcome, LcContractSeedError> {
        let started = Instant::now();
        let lc = self.read_lc_contract().await?;
        let voting_epoch = lc.voting_epoch;
        let log = |outcome: &str| {
            tracing::info!(
                target: "announce::catchup",
                %target,
                held_start = %start,
                lc_contract_voting_epoch = %voting_epoch,
                outcome,
                duration_ms = started.elapsed().as_millis() as u64,
                "lc contract seed"
            );
        };
        if voting_epoch <= start + 1 {
            log("not needed");
            return Ok(LcContractSeedOutcome::NotNeeded);
        }
        if target < voting_epoch {
            return Err(LcContractSeedError::BelowLcContractEpoch {
                target,
                voting_epoch,
            });
        }

        let epoch_height = *coordinator.epoch_height();
        let (seed_header, candidate) = self.fetch_seed_header(&lc, &coordinator).await?;
        let anchor = self.fetch_anchor(&lc, epoch_height, candidate).await?;
        // `start - 1` makes `proven_root_heights` begin above the held pair.
        let held_floor = EpochNumber::new(start.saturating_sub(1));
        let proven = self
            .fetch_proven_headers(&lc, epoch_height, held_floor, &anchor.leaf)
            .await?;
        ensure_seed_header_proven(&seed_header, &proven, voting_epoch)?;

        let mut roots = proven;
        roots.push((voting_epoch + 1, anchor.root_header));
        let membership = coordinator.membership();
        let mut installed = Vec::new();
        let result = async {
            for (epoch, header) in roots {
                if membership.snapshot(epoch).is_some() {
                    continue;
                }
                membership.install_epoch_root(header, &coordinator).await?;
                installed.push(epoch);
            }
            self.check_installed_commitment(&lc, &coordinator)
        }
        .await;
        if let Err(err) = result {
            for epoch in installed {
                membership.remove_epochs(epoch..=epoch);
            }
            return Err(err);
        }

        coordinator.supply_drb(voting_epoch, anchor.drb);
        log("seeded");
        Ok(LcContractSeedOutcome::Seeded {
            start: voting_epoch + 1,
        })
    }

    /// The hook for `EpochMembershipCoordinator::set_lc_contract_seed`.
    pub fn into_seed_fn(self) -> LcContractSeedFn<SeqTypes> {
        let seed = Arc::new(self);
        Arc::new(move |target, start, coordinator| {
            let seed = Arc::clone(&seed);
            async move { Ok(seed.run(target, start, coordinator).await?) }.boxed()
        })
    }

    /// The LC contract state at the L1 finalized block.
    async fn read_lc_contract(&self) -> Result<LcContractVotingStakeTable, LcContractSeedError> {
        let finalized = self.node_state.l1_client.snapshot().await.finalized;
        let l1_block = finalized
            .context("no L1 finalized block yet")
            .map_err(LcContractSeedError::Fetch)?
            .number;
        self.node_state
            .lc_contract_voting_stake_table(l1_block)
            .await
            .map_err(LcContractSeedError::Fetch)
    }

    /// The request-response protocol, if it is up.
    fn requests(&self) -> Result<&RequestResponseProtocol<I, P>, LcContractSeedError> {
        self.requests.get().ok_or_else(|| {
            LcContractSeedError::Fetch(anyhow!("the request-response protocol is not started"))
        })
    }

    /// `root(E'-2)` and the table it derives, the candidate quorum of the anchor: the first
    /// response that passes `SeedHeaderCheck`.
    async fn fetch_seed_header(
        &self,
        lc: &LcContractVotingStakeTable,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<(Header, HSStakeTable<SeqTypes>), LcContractSeedError> {
        let epoch_height = *coordinator.epoch_height();
        let height = root_block_in_epoch(*lc.voting_epoch - 2, epoch_height);
        let check = SeedHeaderCheck {
            height,
            epoch_height,
            capacity: self.stake_table_capacity,
            lc: lc.clone(),
            coordinator: coordinator.clone(),
        };
        let (leaves, table) = self
            .requests()?
            .request_indefinitely(
                Request::Leaf(height),
                move |_: &Request, response: Response| {
                    let check = check.clone();
                    async move {
                        let Response::Leaf(leaves) = response else {
                            bail!("expected leaf response");
                        };
                        tokio::spawn(check.check(leaves))
                            .await?
                            .inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                    }
                },
            )
            .await
            .map_err(|err| LcContractSeedError::Fetch(err.into()))?;
        Ok((leaves[0].block_header().clone(), table))
    }

    /// The chain `root(E'-1) ..= H*(E'-1)+1` under cert2, verified with `quorum` and the restored
    /// upgrade lock. Retried from the leaves on a failed attempt, until the coordinator's timeout.
    async fn fetch_anchor(
        &self,
        lc: &LcContractVotingStakeTable,
        epoch_height: u64,
        quorum: HSStakeTable<SeqTypes>,
    ) -> Result<AnchorChain, LcContractSeedError> {
        let voting_epoch = *lc.voting_epoch;
        let spec = Arc::new(AnchorSpec {
            lowest: root_block_in_epoch(voting_epoch - 1, epoch_height),
            anchor: lc.anchor_height(epoch_height),
            drb: epoch_height * (voting_epoch - 1) - 3,
            epoch_height,
            quorum,
            upgrade_lock: self.upgrade_lock.clone(),
        });
        loop {
            match timeout(ANCHOR_TRY_TIMEOUT, self.try_fetch_anchor(&spec)).await {
                Ok(Ok(chain)) => return spec.anchor_chain(chain),
                Ok(Err(err)) => tracing::debug!("anchor chain attempt failed: {err:#}"),
                Err(_) => tracing::debug!("anchor chain attempt timed out"),
            }
        }
    }

    /// One attempt: `Leaf(h)` for every height of the chain, then the `Cert2` of the newest leaf
    /// whose answer must verify the whole chain.
    async fn try_fetch_anchor(&self, spec: &Arc<AnchorSpec>) -> anyhow::Result<Vec<Leaf2>> {
        let requests = self.requests()?;
        let answers = try_join_all((spec.lowest..=spec.anchor).map(|height| {
            requests.request_indefinitely(
                Request::Leaf(height),
                move |_: &Request, response: Response| async move {
                    let Response::Leaf(leaves) = response else {
                        bail!("expected leaf response");
                    };
                    ensure!(
                        leaves.first().map(Leaf2::height) == Some(height),
                        "leaf chain does not start at height {height}"
                    );
                    Ok(leaves)
                },
            )
        }))
        .await?;
        let leaves: Arc<Vec<Leaf2>> = Arc::new(answers.into_iter().flatten().collect());
        let newest = leaves
            .iter()
            .map(Leaf2::height)
            .max()
            .context("no leaves")?;
        let spec = Arc::clone(spec);
        let path = requests
            .request_indefinitely(
                Request::Cert2(newest),
                move |_: &Request, response: Response| {
                    let (spec, leaves) = (Arc::clone(&spec), Arc::clone(&leaves));
                    async move {
                        let Response::Cert2(cert2) = response else {
                            bail!("expected cert2 response");
                        };
                        ensure!(
                            cert2.data.block_number == newest,
                            "received cert2 at height {} but expected {newest}",
                            cert2.data.block_number
                        );
                        spec.verify(CertifiedLeafChain {
                            leaves: leaves.to_vec(),
                            cert2,
                        })
                        .inspect_err(|err| tracing::debug!("anchor chain rejected: {err:#}"))
                    }
                },
            )
            .await?;
        Ok(path)
    }

    /// The headers of `lc.proven_root_heights(.., held_floor)`, proven against `anchor`, ascending.
    async fn fetch_proven_headers(
        &self,
        lc: &LcContractVotingStakeTable,
        epoch_height: u64,
        held_floor: EpochNumber,
        anchor: &VerifiedAnchorLeaf,
    ) -> Result<Vec<(EpochNumber, Header)>, LcContractSeedError> {
        let requests = self.requests()?;
        let root = anchor.height();
        let anchor = Arc::new(anchor.clone());
        try_join_all(
            lc.proven_root_heights(epoch_height, held_floor)
                .into_iter()
                .map(|(epoch, height)| {
                    let anchor = Arc::clone(&anchor);
                    async move {
                        let header = requests
                            .request_indefinitely(
                                Request::HeaderProof { root, height },
                                move |_: &Request, response: Response| {
                                    let anchor = Arc::clone(&anchor);
                                    async move {
                                        let Response::HeaderProof(proof) = response else {
                                            bail!("expected header proof response");
                                        };
                                        anchor.verify_header_proof(*proof, height).inspect_err(
                                            |err| tracing::debug!("header proof rejected: {err:#}"),
                                        )
                                    }
                                },
                            )
                            .await
                            .map_err(|err| LcContractSeedError::Fetch(err.into()))?;
                        Ok((epoch, header))
                    }
                }),
        )
        .await
    }

    /// The installed `ST(E')` must commit to `votingStakeTableState`.
    fn check_installed_commitment(
        &self,
        lc: &LcContractVotingStakeTable,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<(), LcContractSeedError> {
        let mismatch = LcContractSeedError::CommitmentMismatch {
            epoch: lc.voting_epoch,
        };
        let snapshot = coordinator
            .membership()
            .snapshot(lc.voting_epoch)
            .ok_or_else(|| LcContractSeedError::Fetch(anyhow!("ST(E') is not installed")))?;
        let table = HSStakeTable::<SeqTypes>::from_iter(snapshot.stake_table());
        match lc.commits_to(&table, self.stake_table_capacity) {
            Ok(true) => Ok(()),
            Ok(false) => Err(mismatch),
            Err(err) => Err(LcContractSeedError::Fetch(err)),
        }
    }
}

/// The proven header of `voting_epoch` must be the seed header the candidate quorum came from.
fn ensure_seed_header_proven(
    seed_header: &Header,
    proven: &[(EpochNumber, Header)],
    voting_epoch: EpochNumber,
) -> Result<(), LcContractSeedError> {
    let proven_header = proven
        .iter()
        .find_map(|(epoch, header)| (*epoch == voting_epoch).then_some(header))
        .ok_or_else(|| LcContractSeedError::Fetch(anyhow!("the seed header was not proven")))?;
    if proven_header.commit() != seed_header.commit() {
        return Err(LcContractSeedError::SeedHeaderChanged {
            height: seed_header.height(),
        });
    }
    Ok(())
}

/// Accepts a `Leaf` response that is the epoch root leaf whose L1-derived table commits to the LC
/// contract: right height, an epoch root, the known chain config, and the commitment.
#[derive(Clone)]
struct SeedHeaderCheck {
    height: u64,
    epoch_height: u64,
    capacity: usize,
    lc: LcContractVotingStakeTable,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
}

impl SeedHeaderCheck {
    /// The response and the table derived from its header.
    async fn check(
        self,
        leaves: Vec<Leaf2>,
    ) -> anyhow::Result<(Vec<Leaf2>, HSStakeTable<SeqTypes>)> {
        let leaf = leaves.first().context("empty leaf chain")?;
        let header = leaf.block_header();
        ensure!(
            header.height() == self.height,
            "leaf is at height {}, expected {}",
            header.height(),
            self.height
        );
        ensure!(
            is_epoch_root(header.height(), self.epoch_height),
            "leaf at height {} is not an epoch root",
            header.height()
        );
        let membership = self.coordinator.membership();
        let known = membership.fetcher().known_chain_config().await;
        ensure!(
            header.chain_config().commit() == known.commit(),
            "chain config of the seed header differs from the known one"
        );
        let table = membership.l1_derived_stake_table(header).await?;
        ensure!(
            self.lc.commits_to(&table, self.capacity)?,
            "table derived from the seed header does not match the LC contract"
        );
        Ok((leaves, table))
    }
}

/// Heights and checks of the anchor chain.
struct AnchorSpec {
    /// `root(E'-1)`: the lowest leaf, whose header defines `ST(E'+1)`.
    lowest: u64,
    /// `H*(E'-1)+1`: the anchor.
    anchor: u64,
    /// `H*(E'-1)-3`: the leaf carrying `DRB(E')`.
    drb: u64,
    epoch_height: u64,
    quorum: HSStakeTable<SeqTypes>,
    upgrade_lock: UpgradeLock<SeqTypes>,
}

impl AnchorSpec {
    /// The leaves `lowest ..= newest` under cert2, which must reach the anchor.
    fn verify(&self, chain: CertifiedLeafChain<SeqTypes>) -> anyhow::Result<Vec<Leaf2>> {
        let path = chain.verify_ancestors(
            self.lowest,
            self.epoch_height,
            &self.upgrade_lock,
            &self.quorum,
        )?;
        ensure!(
            path.last().map(Leaf2::height) >= Some(self.anchor),
            "chain ends below the anchor height {}",
            self.anchor
        );
        Ok(path)
    }

    /// The anchor leaf, `root(E'-1)` header and `DRB(E')` out of a verified path.
    fn anchor_chain(&self, path: Vec<Leaf2>) -> Result<AnchorChain, LcContractSeedError> {
        let at = |height: u64| {
            path.iter()
                .find(|leaf| leaf.height() == height)
                .ok_or_else(|| LcContractSeedError::Fetch(anyhow!("no leaf at height {height}")))
        };
        let drb = at(self.drb)?.next_drb_result.ok_or_else(|| {
            LcContractSeedError::Fetch(anyhow!("leaf {} carries no DRB result", self.drb))
        })?;
        Ok(AnchorChain {
            leaf: VerifiedAnchorLeaf::from_verified(at(self.anchor)?.clone()),
            root_header: at(self.lowest)?.block_header().clone(),
            drb,
        })
    }
}

/// What the anchor chain gives.
struct AnchorChain {
    leaf: VerifiedAnchorLeaf,
    /// Header of `root(E'-1)`, defining `ST(E'+1)`.
    root_header: Header,
    /// `DRB(E')`.
    drb: hotshot_types::drb::DrbResult,
}

/// Newest verified leaf; older headers are proven against its block Merkle root.
#[derive(Clone)]
pub(crate) struct VerifiedAnchorLeaf {
    leaf: Leaf2,
}

impl VerifiedAnchorLeaf {
    /// Only constructor: a leaf `CertifiedLeafChain::verify_ancestors` returned.
    pub fn from_verified(leaf: Leaf2) -> Self {
        Self { leaf }
    }

    /// Block height of the leaf.
    pub fn height(&self) -> u64 {
        self.leaf.height()
    }

    /// Header at `expected_height`, if `proof` holds against this leaf's `block_merkle_tree_root`
    /// and proves that height. `HeaderProof::verify` checks the proof against the header it
    /// carries, not against the height requested, so the height check is here.
    pub fn verify_header_proof(
        &self,
        proof: HeaderProof,
        expected_height: u64,
    ) -> anyhow::Result<Header> {
        let header = proof.verify(self.leaf.block_header().block_merkle_tree_root())?;
        ensure!(
            header.height() == expected_height,
            "header proof is for height {}, expected {expected_height}",
            header.height()
        );
        Ok(header)
    }
}

/// Why one seed attempt failed. Each variant is retried on the next catchup trigger.
#[derive(Debug, Error)]
pub(crate) enum LcContractSeedError {
    #[error("catchup target {target} is below the LC contract epoch {voting_epoch}")]
    BelowLcContractEpoch {
        target: EpochNumber,
        voting_epoch: EpochNumber,
    },
    #[error("installed table of epoch {epoch} does not match the LC contract commitment")]
    CommitmentMismatch { epoch: EpochNumber },
    #[error("proven root header at height {height} differs from the seed header")]
    SeedHeaderChanged { height: u64 },
    #[error(transparent)]
    Install(#[from] EpochRootInstallError),
    #[error(transparent)]
    Fetch(anyhow::Error),
}
