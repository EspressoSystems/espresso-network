//! Seeds the post-start epoch catchup from the LC contract.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, bail, ensure};
use committable::{Commitment, Committable};
use espresso_types::{
    ChainConfig, Header, LcContractVotingStakeTable, Leaf2, NodeState, SeqTypes,
    traits::SequencerPersistence,
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream};
use hotshot_new_protocol::{storage::NewProtocolStorage, utils::CertifiedLeafChain};
use hotshot_types::{
    data::EpochNumber,
    drb::DrbResult,
    epoch_membership::{
        EpochMembershipCoordinator, LcContractSeedFn, LcContractSeedOutcome,
        TargetBelowLcContractEpoch,
    },
    message::UpgradeLock,
    simple_certificate::Certificate2,
    stake_table::HSStakeTable,
    traits::node_implementation::NodeImplementation,
    utils::root_block_in_epoch,
};
use light_client::consensus::header::HeaderProof;
use tokio_util::task::AbortOnDropHandle;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// The LC-contract seed of the catchup, behind `LcContractSeedFn`.
///
/// - Trust: `ST(E')` keys and stake come from the SNARK-verified `votingStakeTableState`. The seed
///   header is used only as the candidate quorum for the anchor until it is proven, and must equal
///   the proven `root(E'-2)`. Every installed table comes from a header proven against the anchor
///   `H*(E'-1)+1` or reached from it by parent links.
/// - Proven headers: `root(E'-5) ..= root(E'-2)`, those above the held pair. Chain-linked:
///   `root(E'-1)`, `H*(E'-1)-3` (`DRB(E')`). Installed: the epochs above the held pair within
///   `ST(E'-3) ..= ST(E'+1)`, and `DRB(E')`; rewards of `E' ..= E'+2` are then computable.
///   Epochs between the held pair and `E'-3` stay absent, so a target below `E'` fails fast;
///   `prune_epochs` may evict epochs more than 20 below the newest, which reload from storage.
/// - Rejections: the seed header's chain config must equal the known config or one of the genesis
///   upgrades'; its L1 finalized block must be at or below the L1 block the LC contract was read
///   at; a hash mismatch evicts the epochs this seed created and errors, never panics. Cert checks
///   use the restored upgrade lock.
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
    ) -> anyhow::Result<LcContractSeedOutcome> {
        let started = tokio::time::Instant::now();
        let lc = self.node_state.lc_contract_voting_stake_table().await?;
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
        match seed_decision(voting_epoch, target, start) {
            SeedDecision::NotNeeded => {
                log("not needed");
                return Ok(LcContractSeedOutcome::NotNeeded);
            },
            SeedDecision::TargetBelow => {
                return Err(TargetBelowLcContractEpoch {
                    target,
                    voting_epoch,
                }
                .into());
            },
            SeedDecision::Seed => {},
        }

        let epoch_height = *coordinator.epoch_height();
        let (seed_header, quorum) = self.fetch_seed_header(&lc, &coordinator).await?;
        let checks = Arc::new(AnchorChecks::new(
            &lc,
            epoch_height,
            quorum,
            self.upgrade_lock.clone(),
        ));
        let anchor = self.fetch_anchor(&checks).await?;
        // `start - 1` makes `proven_root_heights` begin above the held pair.
        let held_floor = EpochNumber::new(start.saturating_sub(1));
        let proven = self
            .fetch_proven_headers(&lc, epoch_height, held_floor, &anchor.leaf)
            .await?;
        ensure_seed_header_proven(&seed_header, &proven, voting_epoch)?;

        let roots = proven
            .into_iter()
            .map(|(_, header)| header)
            .chain([anchor.root_header])
            .collect();
        coordinator
            .membership()
            .install_epoch_roots(roots, &coordinator)
            .await?;

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
            async move { seed.run(target, start, coordinator).await }.boxed()
        })
    }

    /// The request-response protocol, if it is up.
    fn requests(&self) -> anyhow::Result<&RequestResponseProtocol<I, P>> {
        self.requests
            .get()
            .context("the request-response protocol is not started")
    }

    /// `root(E'-2)` and the table it derives, the candidate quorum of the anchor: the first
    /// response that passes `SeedHeaderCheck`.
    async fn fetch_seed_header(
        &self,
        lc: &LcContractVotingStakeTable,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<(Header, HSStakeTable<SeqTypes>)> {
        let height = root_block_in_epoch(*lc.voting_epoch - 2, *coordinator.epoch_height());
        let upgrade_configs = self
            .node_state
            .upgrades
            .values()
            .filter_map(|upgrade| upgrade.upgrade_type.chain_config())
            .map(|config| config.commit())
            .collect();
        let check = SeedHeaderCheck {
            height,
            capacity: self.stake_table_capacity,
            lc: lc.clone(),
            coordinator: coordinator.clone(),
            upgrade_configs,
            warned: Arc::default(),
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
                        // The check reads L1, whose futures are not `Sync` as the validator's
                        // must be; the task is aborted if the request is dropped.
                        AbortOnDropHandle::new(tokio::spawn(check.check(leaves)))
                            .await?
                            .inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                    }
                },
            )
            .await?;
        Ok((leaves[0].block_header().clone(), table))
    }

    /// The chain `root(E'-1) ..= newest` under cert2 with the candidate quorum, where `newest` is
    /// the leaf the cert2 of the `Leaf(anchor)` answer commits to. The leaves below the anchor
    /// are fetched one height at a time, each linked to the one above it by its parent
    /// commitment.
    async fn fetch_anchor(&self, checks: &Arc<AnchorChecks>) -> anyhow::Result<AnchorChain> {
        let requests = self.requests()?;
        let leaves = {
            let checks = Arc::clone(checks);
            requests
                .request_indefinitely(
                    Request::Leaf(checks.anchor),
                    move |_: &Request, response: Response| {
                        let checks = Arc::clone(&checks);
                        async move {
                            let Response::Leaf(leaves) = response else {
                                bail!("expected leaf response");
                            };
                            checks.check_leaves(leaves).inspect_err(|err| {
                                tracing::debug!("anchor leaves rejected: {err:#}")
                            })
                        }
                    },
                )
                .await?
        };
        let newest = leaves.last().context("no leaves")?.height();
        let mut path = {
            let (checks, leaves) = (Arc::clone(checks), leaves);
            requests
                .request_indefinitely(
                    Request::Cert2(newest),
                    move |_: &Request, response: Response| {
                        let (checks, leaves) = (Arc::clone(&checks), leaves.clone());
                        async move {
                            let Response::Cert2(cert2) = response else {
                                bail!("expected cert2 response");
                            };
                            ensure!(
                                cert2.data.block_number == newest,
                                "received cert2 at height {} but expected {newest}",
                                cert2.data.block_number
                            );
                            checks.check_cert2(leaves, cert2).inspect_err(|err| {
                                tracing::debug!("anchor cert2 rejected: {err:#}")
                            })
                        }
                    },
                )
                .await?
        };
        for height in (checks.lowest..checks.anchor).rev() {
            let child = path.first().context("empty chain")?;
            let parent = self.fetch_parent(height, child).await?;
            path.insert(0, parent);
        }
        checks.anchor_chain(&path)
    }

    /// The leaf at `height`, which must be the parent of `child`.
    async fn fetch_parent(&self, height: u64, child: &Leaf2) -> anyhow::Result<Leaf2> {
        let expected = child.parent_commitment();
        Ok(self
            .requests()?
            .request_indefinitely(
                Request::Leaf(height),
                move |_: &Request, response: Response| async move {
                    let Response::Leaf(leaves) = response else {
                        bail!("expected leaf response");
                    };
                    ensure_parent(leaves, height, expected)
                        .inspect_err(|err| tracing::debug!("parent leaf rejected: {err:#}"))
                },
            )
            .await?)
    }

    /// The headers of `lc.proven_root_heights(.., held_floor)`, proven against `anchor`,
    /// ascending.
    async fn fetch_proven_headers(
        &self,
        lc: &LcContractVotingStakeTable,
        epoch_height: u64,
        held_floor: EpochNumber,
        anchor: &VerifiedAnchorLeaf,
    ) -> anyhow::Result<Vec<(EpochNumber, Header)>> {
        stream::iter(lc.proven_root_heights(epoch_height, held_floor))
            .map(|(epoch, height)| async move {
                let header = self.fetch_header_proof(anchor, height).await?;
                anyhow::Ok((epoch, header))
            })
            .buffered(MAX_CONCURRENT_REQUESTS)
            .try_collect()
            .await
    }

    /// The header at `height`, proven against `anchor`.
    async fn fetch_header_proof(
        &self,
        anchor: &VerifiedAnchorLeaf,
        height: u64,
    ) -> anyhow::Result<Header> {
        let root = anchor.height();
        let anchor = anchor.clone();
        Ok(self
            .requests()?
            .request_indefinitely(
                Request::HeaderProof { root, height },
                move |_: &Request, response: Response| {
                    let anchor = anchor.clone();
                    async move {
                        let Response::HeaderProof(proof) = response else {
                            bail!("expected header proof response");
                        };
                        anchor
                            .verify_header_proof(*proof, height)
                            .inspect_err(|err| tracing::debug!("header proof rejected: {err:#}"))
                    }
                },
            )
            .await?)
    }
}

/// What a seed attempt does for `catchup(target)` with held pair top `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeedDecision {
    /// The LC contract epoch is at most `start + 1`: the walk runs unchanged.
    NotNeeded,
    /// The target is below the LC contract epoch: fail fast.
    TargetBelow,
    /// Install the window of the LC contract epoch.
    Seed,
}

/// The decision for the LC contract epoch `voting_epoch`. Pure.
fn seed_decision(
    voting_epoch: EpochNumber,
    target: EpochNumber,
    start: EpochNumber,
) -> SeedDecision {
    if voting_epoch <= start + 1 {
        SeedDecision::NotNeeded
    } else if target < voting_epoch {
        SeedDecision::TargetBelow
    } else {
        SeedDecision::Seed
    }
}

/// The proven header of `voting_epoch` must be the seed header the candidate quorum came from.
fn ensure_seed_header_proven(
    seed_header: &Header,
    proven: &[(EpochNumber, Header)],
    voting_epoch: EpochNumber,
) -> anyhow::Result<()> {
    let proven_header = proven
        .iter()
        .find_map(|(epoch, header)| (*epoch == voting_epoch).then_some(header))
        .context("the seed header was not proven")?;
    ensure!(
        proven_header.commit() == seed_header.commit(),
        "proven root header at height {} differs from the seed header",
        seed_header.height()
    );
    Ok(())
}

/// A header whose L1 finalized block is above `l1_block`, the block the LC contract was read at,
/// may be unfinalized or in the future: deriving its table would scan L1 past what is complete.
fn ensure_l1_finalized_within(header: &Header, l1_block: u64) -> anyhow::Result<()> {
    let finalized = header
        .l1_finalized()
        .context("seed header has no L1 finalized block")?;
    ensure!(
        finalized.number() <= l1_block,
        "seed header finalizes L1 block {}, above the LC contract read block {l1_block}",
        finalized.number()
    );
    Ok(())
}

/// The leaf at `height` out of a `Leaf(height)` answer, if it is `expected`.
fn ensure_parent(
    leaves: Vec<Leaf2>,
    height: u64,
    expected: Commitment<Leaf2>,
) -> anyhow::Result<Leaf2> {
    let leaf = leaves.into_iter().next().context("empty leaf chain")?;
    ensure!(
        leaf.height() == height,
        "leaf is at height {}, expected {height}",
        leaf.height()
    );
    ensure!(
        leaf.commit() == expected,
        "leaf at height {height} is not the parent commitment of its child"
    );
    Ok(leaf)
}

/// Accepts a `Leaf` response that is the epoch root leaf whose L1-derived table commits to the LC
/// contract: right height, finalized L1 within the LC contract read, a known chain config, and the
/// commitment.
#[derive(Clone)]
struct SeedHeaderCheck {
    height: u64,
    capacity: usize,
    lc: LcContractVotingStakeTable,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    /// Chain configs of the genesis upgrades, accepted besides the known one.
    upgrade_configs: Vec<Commitment<ChainConfig>>,
    /// Whether a rejection for the chain config was logged at WARN.
    warned: Arc<AtomicBool>,
}

impl SeedHeaderCheck {
    /// The response and the table derived from its header.
    async fn check(
        self,
        leaves: Vec<Leaf2>,
    ) -> anyhow::Result<(Vec<Leaf2>, HSStakeTable<SeqTypes>)> {
        let header = leaves.first().context("empty leaf chain")?.block_header();
        ensure!(
            header.height() == self.height,
            "leaf is at height {}, expected {}",
            header.height(),
            self.height
        );
        ensure_l1_finalized_within(header, self.lc.l1_block)?;
        let membership = self.coordinator.membership();
        let known = membership.fetcher().known_chain_config().await;
        let config = header.chain_config().commit();
        if config != known.commit() && !self.upgrade_configs.contains(&config) {
            if !self.warned.swap(true, Ordering::Relaxed) {
                tracing::warn!(
                    target: "announce::catchup",
                    "a seed header has a chain config unknown to this node"
                );
            }
            bail!("chain config of the seed header differs from the known ones");
        }
        let table = membership.l1_derived_stake_table(header).await?;
        ensure!(
            self.lc.commits_to(&table, self.capacity)?,
            "table derived from the seed header does not match the LC contract"
        );
        Ok((leaves, table))
    }
}

/// Heights and checks of the anchor chain.
struct AnchorChecks {
    /// `root(E'-1)`: the lowest leaf, whose header defines `ST(E'+1)`.
    lowest: u64,
    /// `H*(E'-1)+1`: the anchor.
    anchor: u64,
    /// `H*(E'-1)-3`: the leaf carrying `DRB(E')`.
    drb: u64,
    voting_epoch: EpochNumber,
    epoch_height: u64,
    quorum: HSStakeTable<SeqTypes>,
    upgrade_lock: UpgradeLock<SeqTypes>,
}

impl AnchorChecks {
    /// The checks of the anchor chain of `lc`, under the candidate `quorum` of `E'`.
    fn new(
        lc: &LcContractVotingStakeTable,
        epoch_height: u64,
        quorum: HSStakeTable<SeqTypes>,
        upgrade_lock: UpgradeLock<SeqTypes>,
    ) -> Self {
        let voting_epoch = *lc.voting_epoch;
        Self {
            lowest: root_block_in_epoch(voting_epoch - 1, epoch_height),
            anchor: lc.anchor_height(epoch_height),
            drb: epoch_height * (voting_epoch - 1) - 3,
            voting_epoch: lc.voting_epoch,
            epoch_height,
            quorum,
            upgrade_lock,
        }
    }

    /// The leaves of a `Leaf(anchor)` answer: they start at the anchor and each is the parent of
    /// the next, so a leaf that is off the chain is rejected before its cert2 is asked for.
    fn check_leaves(&self, leaves: Vec<Leaf2>) -> anyhow::Result<Vec<Leaf2>> {
        ensure!(
            leaves.first().map(Leaf2::height) == Some(self.anchor),
            "leaf chain does not start at the anchor height {}",
            self.anchor
        );
        for pair in leaves.windows(2) {
            ensure!(
                pair[1].parent_commitment() == pair[0].commit()
                    && pair[0].height() + 1 == pair[1].height(),
                "leaf chain is not linked at height {}",
                pair[0].height()
            );
        }
        Ok(leaves)
    }

    /// The leaves of `check_leaves` under `cert2`, signed by `E'` as the candidate quorum says.
    fn check_cert2(
        &self,
        leaves: Vec<Leaf2>,
        cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<Vec<Leaf2>> {
        ensure!(
            cert2.data.epoch == self.voting_epoch,
            "cert2 is of epoch {}, expected {}",
            cert2.data.epoch,
            self.voting_epoch
        );
        CertifiedLeafChain { leaves, cert2 }.verify_ancestors(
            self.anchor,
            self.epoch_height,
            &self.upgrade_lock,
            &self.quorum,
        )
    }

    /// The anchor leaf, the `root(E'-1)` header and `DRB(E')` of the verified `path`, which runs
    /// from `lowest` up to the cert2 leaf.
    fn anchor_chain(&self, path: &[Leaf2]) -> anyhow::Result<AnchorChain> {
        let at = |height: u64| {
            path.iter()
                .find(|leaf| leaf.height() == height)
                .with_context(|| format!("no leaf at height {height}"))
        };
        let drb = at(self.drb)?
            .next_drb_result
            .with_context(|| format!("leaf {} carries no DRB result", self.drb))?;
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
    drb: DrbResult,
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

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use espresso_types::{L1BlockInfo, StakeTableCommitment};
    use hotshot_new_protocol::testing::{sign_leaf_chain, with_next_drb_result};
    use hotshot_query_service::node::BlockId;
    use hotshot_types::{
        PeerConfig, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
    };
    use light_client::{
        client::Client,
        testing::{TestClient, leaf_chain},
    };
    use rstest::rstest;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    /// With `E' = 4`: `lowest = 25`, `drb = 27`, `anchor = 31`.
    const VOTING_EPOCH: u64 = 4;
    const DRB: DrbResult = [7; 32];

    fn epoch(epoch: u64) -> EpochNumber {
        EpochNumber::new(epoch)
    }

    #[rstest]
    // The LC contract epoch is the next table already held.
    #[case(4, 9, 3, SeedDecision::NotNeeded)]
    #[case(2, 9, 3, SeedDecision::NotNeeded)]
    // Below the LC contract epoch.
    #[case(6, 5, 2, SeedDecision::TargetBelow)]
    // At it.
    #[case(6, 6, 2, SeedDecision::Seed)]
    #[case(6, 9, 2, SeedDecision::Seed)]
    // The held pair is the seeded genesis pair.
    #[case(3, 3, 2, SeedDecision::NotNeeded)]
    #[case(4, 4, 2, SeedDecision::Seed)]
    fn seed_decision_cases(
        #[case] voting_epoch: u64,
        #[case] target: u64,
        #[case] start: u64,
        #[case] expected: SeedDecision,
    ) {
        assert_eq!(
            seed_decision(epoch(voting_epoch), epoch(target), epoch(start)),
            expected
        );
    }

    fn signers() -> Vec<(BLSPubKey, <BLSPubKey as SignatureKey>::PrivateKey)> {
        (0..4)
            .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i))
            .collect()
    }

    fn quorum() -> HSStakeTable<SeqTypes> {
        signers()
            .iter()
            .map(|(key, _)| PeerConfig::<SeqTypes> {
                stake_table_entry: key.stake_table_entry(U256::ONE),
                state_ver_key: Default::default(),
                connect_info: None,
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn lock() -> UpgradeLock<SeqTypes> {
        UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
    }

    fn checks(voting_epoch: u64) -> AnchorChecks {
        let lc = LcContractVotingStakeTable {
            l1_block: 0,
            finalized_block_height: 0,
            voting_epoch: epoch(voting_epoch),
            commitment: StakeTableCommitment::default(),
        };
        AnchorChecks::new(&lc, EPOCH_HEIGHT, quorum(), lock())
    }

    /// Leaves `25 ..= 33` linked and closed by a cert2 of the quorum, `DRB` in the leaf at 27.
    async fn signed_chain() -> CertifiedLeafChain<SeqTypes> {
        let leaves = leaf_chain(25..=33, NEW_PROTOCOL_VERSION)
            .await
            .into_iter()
            .map(|leaf| match leaf.leaf().clone() {
                leaf if leaf.height() == 27 => with_next_drb_result(&leaf, DRB),
                leaf => leaf,
            })
            .collect();
        sign_leaf_chain(leaves, EPOCH_HEIGHT, &signers(), &lock())
    }

    /// The leaves of `chain` from `height` up.
    fn from_height(chain: &CertifiedLeafChain<SeqTypes>, height: u64) -> Vec<Leaf2> {
        chain
            .leaves
            .iter()
            .filter(|leaf| leaf.height() >= height)
            .cloned()
            .collect()
    }

    #[tokio::test]
    async fn anchor_chain_extracts_drb_and_headers_ok() {
        let checks = checks(VOTING_EPOCH);
        let chain = signed_chain().await;

        let leaves = checks.check_leaves(from_height(&chain, 31)).unwrap();
        let above = checks.check_cert2(leaves, chain.cert2.clone()).unwrap();
        let path = [from_height(&chain, 25)[..6].to_vec(), above].concat();
        let anchor = checks.anchor_chain(&path).unwrap();

        assert_eq!(anchor.drb, DRB);
        assert_eq!(anchor.leaf.height(), 31);
        assert_eq!(anchor.root_header.height(), 25);
    }

    #[tokio::test]
    async fn anchor_chain_ending_below_anchor_fails() {
        let checks = checks(VOTING_EPOCH);
        let chain = signed_chain().await;

        let below: Vec<_> = chain
            .leaves
            .iter()
            .filter(|l| l.height() < 31)
            .cloned()
            .collect();

        assert!(checks.anchor_chain(&below).is_err());
        checks.check_leaves(below).unwrap_err();
    }

    /// A leaf that is not the child of the previous one, of a higher view, from one fast peer.
    #[tokio::test]
    async fn junk_high_view_leaf_is_rejected_before_its_cert2_is_asked_for() {
        let checks = checks(VOTING_EPOCH);
        let chain = signed_chain().await;
        let mut leaves = from_height(&chain, 31);
        leaves.extend(
            leaf_chain(34..=34, NEW_PROTOCOL_VERSION)
                .await
                .into_iter()
                .map(|leaf| leaf.leaf().clone()),
        );

        checks.check_leaves(leaves).unwrap_err();
    }

    #[tokio::test]
    async fn cert2_of_another_epoch_fails() {
        let chain = signed_chain().await;

        let checks = checks(VOTING_EPOCH + 1);

        checks
            .check_cert2(from_height(&chain, 31), chain.cert2)
            .unwrap_err();
    }

    #[tokio::test]
    async fn ensure_parent_cases() {
        let chain = signed_chain().await;
        let child = chain.leaves.iter().find(|l| l.height() == 31).unwrap();
        let parent = chain.leaves.iter().find(|l| l.height() == 30).unwrap();
        let other = chain.leaves.iter().find(|l| l.height() == 29).unwrap();

        ensure_parent(vec![parent.clone()], 30, child.parent_commitment()).unwrap();
        ensure_parent(vec![other.clone()], 30, child.parent_commitment()).unwrap_err();
        ensure_parent(vec![other.clone()], 29, child.parent_commitment()).unwrap_err();
        ensure_parent(vec![], 30, child.parent_commitment()).unwrap_err();
    }

    /// An L1 block above the LC contract read may be unfinalized: the header is refused.
    #[tokio::test]
    async fn l1_finalized_above_lc_contract_read_fails() {
        let client = TestClient::default();
        let mut header = client.leaf(7).await.leaf().block_header().clone();
        *header.l1_finalized_mut() = Some(L1BlockInfo {
            number: 100,
            ..Default::default()
        });

        ensure_l1_finalized_within(&header, 100).unwrap();
        ensure_l1_finalized_within(&header, 101).unwrap();
        ensure_l1_finalized_within(&header, 99).unwrap_err();
        *header.l1_finalized_mut() = None;
        ensure_l1_finalized_within(&header, 100).unwrap_err();
    }

    #[tokio::test]
    async fn header_proof_of_another_height_fails() {
        let client = TestClient::default();
        let anchor = VerifiedAnchorLeaf::from_verified(client.leaf(7).await.leaf().clone());
        let proof = client.header_proof(7, BlockId::Number(3)).await.unwrap();

        anchor.verify_header_proof(proof.clone(), 3).unwrap();
        anchor.verify_header_proof(proof, 4).unwrap_err();
    }
}
