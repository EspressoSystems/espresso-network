//! Seeds the post-start epoch catchup from the LC contract.

use std::sync::{Arc, OnceLock};

use anyhow::{Context, bail, ensure};
use committable::{Commitment, Committable};
use espresso_types::{
    Certificate2, Header, LcContractVotingStakeTable, Leaf2, NodeState, SeqTypes,
    traits::SequencerPersistence,
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream};
use hotshot_new_protocol::{storage::NewProtocolStorage, utils::verify_cert2};
use hotshot_types::{
    data::EpochNumber,
    drb::DrbResult,
    epoch_membership::{EpochMembershipCoordinator, LcContractSeedFn},
    message::UpgradeLock,
    stake_table::HSStakeTable,
    traits::node_implementation::NodeImplementation,
    utils::{root_block_in_epoch, transition_block_for_epoch},
};
use light_client::consensus::header::HeaderProof;
use tokio_util::task::AbortOnDropHandle;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// The LC-contract seed of the catchup, behind `LcContractSeedFn`; steps in `doc/agents/architecture.md`.
///
/// Trust: `ST(E')` keys and stake come from the SNARK-verified `votingStakeTableState`. The seed
/// header only gives the candidate quorum that verifies the anchor `H*(E'-1)+1`; every installed
/// table comes from a header proven against the anchor or reached from it by parent links.
pub(crate) struct LcContractSeed<I: NodeImplementation<SeqTypes>, P: SequencerPersistence>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    node_state: NodeState,
    upgrade_lock: UpgradeLock<SeqTypes>,
    stake_table_capacity: usize,
    /// Set once the request-response protocol exists, which needs the consensus handle built
    /// after the hook is installed. The hook must be installed first: `apply_epoch` at startup
    /// can start a catchup, which then runs without a seed if none is set. An attempt before the
    /// protocol is set fails and is retried.
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

    /// One seed attempt for `catchup(target)` with held pair top `start`: the walk start.
    pub async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<EpochNumber> {
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
                return Ok(start);
            },
            SeedDecision::TargetBelow => {
                bail!("catchup target {target} is below the LC contract epoch {voting_epoch}")
            },
            SeedDecision::Seed => {},
        }

        let epoch_height = *coordinator.epoch_height();
        let quorum = self.fetch_seed_header_table(&lc, &coordinator).await?;
        let checks = Arc::new(AnchorChecks::new(
            &lc,
            epoch_height,
            quorum,
            self.upgrade_lock.clone(),
        ));
        let anchor = self.fetch_anchor(&checks).await?;
        let held_floor = EpochNumber::new(start.saturating_sub(1));
        let proven = self
            .fetch_proven_headers(&lc, epoch_height, held_floor, &anchor.leaf)
            .await?;

        let roots = proven.into_iter().chain([anchor.root_header]).collect();
        coordinator
            .membership()
            .install_epoch_roots(roots, &coordinator)
            .await?;

        coordinator.supply_drb(voting_epoch, anchor.drb);
        log("seeded");
        Ok(voting_epoch + 1)
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

    /// The table `root(E'-2)` derives, the candidate quorum of the anchor: from the first
    /// response that passes `check_seed_header`.
    async fn fetch_seed_header_table(
        &self,
        lc: &LcContractVotingStakeTable,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<HSStakeTable<SeqTypes>> {
        let height = root_block_in_epoch(*lc.voting_epoch - 2, *coordinator.epoch_height());
        let (lc, coordinator, capacity) =
            (lc.clone(), coordinator.clone(), self.stake_table_capacity);
        Ok(self
            .requests()?
            .request_indefinitely(
                Request::Leaf(height),
                move |_: &Request, response: Response| {
                    let (lc, coordinator) = (lc.clone(), coordinator.clone());
                    async move {
                        let Response::Leaf(leaves) = response else {
                            bail!("expected leaf response");
                        };
                        // The check reads L1, whose futures are not `Sync` as the validator's
                        // must be; the task is aborted if the request is dropped.
                        AbortOnDropHandle::new(tokio::spawn(check_seed_header(
                            leaves,
                            height,
                            lc,
                            capacity,
                            coordinator,
                        )))
                        .await?
                        .inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                    }
                },
            )
            .await?)
    }

    /// The chain `root(E'-1) ..= newest` under cert2 with the candidate quorum, where `newest` is
    /// the last leaf of the `Leaf(anchor)` answer. The leaves below the anchor are fetched one
    /// height at a time, each linked to the one above it by its parent commitment.
    async fn fetch_anchor(&self, checks: &Arc<AnchorChecks>) -> anyhow::Result<AnchorChain> {
        let requests = self.requests()?;
        let mut path = {
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
        let newest = path[path.len() - 1].clone();
        let verifier = Arc::clone(checks);
        requests
            .request_cert2(newest.height(), move |cert2| {
                verifier
                    .check_cert2(&newest, cert2)
                    .inspect_err(|err| tracing::debug!("anchor cert2 rejected: {err:#}"))
            })
            .await?;
        for height in (checks.lowest..checks.anchor).rev() {
            let parent = self.fetch_parent(height, &path[0]).await?;
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
        anchor: &Leaf2,
    ) -> anyhow::Result<Vec<Header>> {
        stream::iter(lc.proven_root_heights(epoch_height, held_floor))
            .map(|(_, height)| self.fetch_header_proof(anchor, height))
            .buffered(MAX_CONCURRENT_REQUESTS)
            .try_collect()
            .await
    }

    /// The header at `height`, proven against `anchor`.
    async fn fetch_header_proof(&self, anchor: &Leaf2, height: u64) -> anyhow::Result<Header> {
        let anchor = anchor.clone();
        Ok(self
            .requests()?
            .request_indefinitely(
                Request::HeaderProof {
                    root: anchor.height(),
                    height,
                },
                move |_: &Request, response: Response| {
                    let anchor = anchor.clone();
                    async move {
                        let Response::HeaderProof(proof) = response else {
                            bail!("expected header proof response");
                        };
                        verify_header_proof(&anchor, *proof, height)
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

/// The table derived from the header of a `Leaf(height)` answer, if that header is the epoch root
/// of `lc`: at `height`, finalizing L1 within the LC contract read, and its table commits to the
/// LC contract.
async fn check_seed_header(
    leaves: Vec<Leaf2>,
    height: u64,
    lc: LcContractVotingStakeTable,
    capacity: usize,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
) -> anyhow::Result<HSStakeTable<SeqTypes>> {
    let header = leaves.first().context("empty leaf chain")?.block_header();
    ensure!(
        header.height() == height,
        "leaf is at height {}, expected {height}",
        header.height()
    );
    ensure_l1_finalized_within(header, lc.l1_block)?;
    let table = coordinator
        .membership()
        .l1_derived_stake_table(header)
        .await?;
    ensure!(
        table.commitment(capacity)? == lc.commitment,
        "table derived from the seed header does not match the LC contract"
    );
    Ok(table)
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
        let previous = *lc.voting_epoch - 1;
        Self {
            lowest: root_block_in_epoch(previous, epoch_height),
            anchor: epoch_height * previous + 1,
            drb: transition_block_for_epoch(previous, epoch_height),
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

    /// `cert2` finalizes `newest`, the last leaf of `check_leaves`, and is of `E'`, signed as the
    /// candidate quorum says.
    fn check_cert2(&self, newest: &Leaf2, cert2: &Certificate2<SeqTypes>) -> anyhow::Result<()> {
        ensure!(
            cert2.data.epoch == self.voting_epoch,
            "cert2 is of epoch {}, expected {}",
            cert2.data.epoch,
            self.voting_epoch
        );
        verify_cert2(
            cert2,
            newest,
            self.epoch_height,
            &self.upgrade_lock,
            &self.quorum,
        )
    }

    /// The anchor leaf, the `root(E'-1)` header and `DRB(E')` of the verified `path`, which runs
    /// from `lowest` without gaps up to the anchor at least.
    fn anchor_chain(&self, path: &[Leaf2]) -> anyhow::Result<AnchorChain> {
        let at = |height: u64| &path[(height - self.lowest) as usize];
        let drb = at(self.drb)
            .next_drb_result
            .with_context(|| format!("leaf {} carries no DRB result", self.drb))?;
        Ok(AnchorChain {
            leaf: at(self.anchor).clone(),
            root_header: at(self.lowest).block_header().clone(),
            drb,
        })
    }
}

/// What the anchor chain gives.
struct AnchorChain {
    /// The anchor leaf: older headers are proven against its block Merkle root.
    leaf: Leaf2,
    /// Header of `root(E'-1)`, defining `ST(E'+1)`.
    root_header: Header,
    /// `DRB(E')`.
    drb: DrbResult,
}

/// The header at `expected_height`, if `proof` holds against the block Merkle root of `anchor`.
/// `HeaderProof::verify` checks the proof against the header it carries, not against the height
/// requested, so the height check is here.
fn verify_header_proof(
    anchor: &Leaf2,
    proof: HeaderProof,
    expected_height: u64,
) -> anyhow::Result<Header> {
    let header = proof.verify(anchor.block_header().block_merkle_tree_root())?;
    ensure!(
        header.height() == expected_height,
        "header proof is for height {}, expected {expected_height}",
        header.height()
    );
    Ok(header)
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use espresso_types::L1BlockInfo;
    use hotshot_query_service::node::BlockId;
    use hotshot_types::{
        light_client::StakeTableState as StakeTableCommitment, simple_vote::Vote2Data,
    };
    use light_client::{
        client::Client,
        testing::{TestClient, custom_leaf_chain, leaf_chain},
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

    fn checks(voting_epoch: u64) -> AnchorChecks {
        let lc = LcContractVotingStakeTable {
            l1_block: 0,
            voting_epoch: epoch(voting_epoch),
            commitment: StakeTableCommitment::default(),
        };
        let lock = UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));
        AnchorChecks::new(&lc, EPOCH_HEIGHT, HSStakeTable::default(), lock)
    }

    /// Linked leaves `25 ..= 33`, `DRB` in the leaf at 27.
    async fn chain() -> Vec<Leaf2> {
        custom_leaf_chain(
            Upgrade::trivial(NEW_PROTOCOL_VERSION),
            25..=33,
            |proposal| {
                if proposal.block_header.height() == 27 {
                    proposal.next_drb_result = Some(DRB);
                }
            },
        )
        .await
        .into_iter()
        .map(|leaf| leaf.leaf().clone())
        .collect()
    }

    fn from_height(chain: &[Leaf2], height: u64) -> Vec<Leaf2> {
        chain
            .iter()
            .filter(|leaf| leaf.height() >= height)
            .cloned()
            .collect()
    }

    #[test]
    fn anchor_checks_heights() {
        let checks = checks(VOTING_EPOCH);

        assert_eq!(
            (checks.lowest, checks.drb, checks.anchor),
            (25, 27, 31),
            "root(E'-1), DRB leaf, anchor"
        );
    }

    #[tokio::test]
    async fn anchor_chain_extracts_drb_and_headers_ok() {
        let checks = checks(VOTING_EPOCH);
        let chain = chain().await;

        checks.check_leaves(from_height(&chain, 31)).unwrap();
        let anchor = checks.anchor_chain(&chain).unwrap();

        assert_eq!(anchor.drb, DRB);
        assert_eq!(anchor.leaf.height(), 31);
        assert_eq!(anchor.root_header.height(), 25);
    }

    #[tokio::test]
    async fn anchor_chain_without_drb_fails() {
        let checks = checks(VOTING_EPOCH);
        let chain = leaf_chain(25..=33, NEW_PROTOCOL_VERSION).await;
        let chain: Vec<_> = chain.iter().map(|leaf| leaf.leaf().clone()).collect();

        checks.anchor_chain(&chain).map(|_| ()).unwrap_err();
    }

    /// A leaf that is not the child of the previous one, of a higher view, from one fast peer.
    #[tokio::test]
    async fn junk_high_view_leaf_is_rejected_before_its_cert2_is_asked_for() {
        let checks = checks(VOTING_EPOCH);
        let mut leaves = from_height(&chain().await, 31);
        leaves.extend(
            leaf_chain(34..=34, NEW_PROTOCOL_VERSION)
                .await
                .into_iter()
                .map(|leaf| leaf.leaf().clone()),
        );

        checks.check_leaves(leaves).unwrap_err();
    }

    #[tokio::test]
    async fn leaves_not_starting_at_the_anchor_fail() {
        let checks = checks(VOTING_EPOCH);

        checks
            .check_leaves(from_height(&chain().await, 30))
            .unwrap_err();
    }

    #[tokio::test]
    async fn cert2_of_another_epoch_fails() {
        let checks = checks(VOTING_EPOCH);
        let newest = chain().await.pop().unwrap();
        let data = Vote2Data {
            leaf_commit: newest.commit(),
            epoch: epoch(VOTING_EPOCH + 1),
            block_number: newest.height(),
        };
        let cert2 = Certificate2::<SeqTypes>::new(
            data.clone(),
            data.commit(),
            newest.view_number(),
            None,
            PhantomData,
        );

        let err = checks.check_cert2(&newest, &cert2).unwrap_err();
        assert!(err.to_string().contains("expected 4"), "{err:#}");
    }

    #[tokio::test]
    async fn ensure_parent_cases() {
        let chain = chain().await;
        let child = chain.iter().find(|l| l.height() == 31).unwrap();
        let parent = chain.iter().find(|l| l.height() == 30).unwrap();
        let other = chain.iter().find(|l| l.height() == 29).unwrap();

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
        let anchor = client.leaf(7).await.leaf().clone();
        let proof = client.header_proof(7, BlockId::Number(3)).await.unwrap();

        verify_header_proof(&anchor, proof.clone(), 3).unwrap();
        verify_header_proof(&anchor, proof, 4).unwrap_err();
    }
}
