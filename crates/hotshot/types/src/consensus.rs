// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the core consensus types

use std::collections::{BTreeMap, HashMap, HashSet};

use alloy_primitives::U256;
use committable::Commitment;
use hotshot_utils::anytrace::*;

pub use crate::utils::{View, ViewInner};
use crate::{
    constants::EPOCH_PARTICIPATION_HISTORY,
    data::{EpochNumber, Leaf2, VidDisperseShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal,
    simple_certificate::QuorumCertificate2,
    simple_vote::HasEpoch,
    stake_table::{HSStakeTable, StakeTableEntries},
    traits::{
        BlockPayload,
        metrics::{Counter, Gauge, Histogram, Metrics, NoMetrics},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    vote::{Certificate, HasViewNumber},
};

/// A type alias for `HashMap<Commitment<T>, T>`
pub type CommitmentMap<T> = HashMap<Commitment<T>, T>;

/// A type alias for `BTreeMap<T::Time, HashMap<T::SignatureKey, BTreeMap<T::Epoch, Proposal<T, VidDisperseShare<T>>>>>`
pub type VidShares<TYPES> = BTreeMap<
    ViewNumber,
    HashMap<
        <TYPES as NodeType>::SignatureKey,
        BTreeMap<Option<EpochNumber>, Proposal<TYPES, VidDisperseShare<TYPES>>>,
    >,
>;

type ValidatorParticipationMap<TYPES> = HashMap<<TYPES as NodeType>::SignatureKey, (u64, u64)>;

/// Tracks proposal and vote participation per validator across epochs.
///
/// The new-protocol coordinator owns one of these and serves the node
/// participation API from it.
pub struct ParticipationTracker<TYPES: NodeType> {
    validator: ValidatorParticipation<TYPES>,
    vote: VoteParticipation<TYPES>,
}

impl<TYPES: NodeType> Default for ParticipationTracker<TYPES> {
    fn default() -> Self {
        let (stake_table, success_threshold) = VoteParticipation::unresolved_stake_table();
        Self {
            validator: ValidatorParticipation::new(),
            vote: VoteParticipation::new(stake_table, success_threshold, None),
        }
    }
}

impl<TYPES: NodeType> ParticipationTracker<TYPES> {
    pub fn new(membership: &EpochMembershipCoordinator<TYPES>, epoch: EpochNumber) -> Self {
        let (stake_table, success_threshold) =
            resolve_participation_stake_table(membership, Some(epoch));
        Self {
            validator: ValidatorParticipation::new_in_epoch(epoch),
            vote: VoteParticipation::new(stake_table, success_threshold, Some(epoch)),
        }
    }

    pub fn leader_proposed(&mut self, leader: TYPES::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, true);
    }

    pub fn leader_missed(&mut self, leader: TYPES::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, false);
    }

    pub fn on_view_changed(&mut self, epoch: EpochNumber) {
        self.validator.update_participation_epoch(epoch);
    }

    pub fn on_leaf_decided(
        &mut self,
        leaf: &Leaf2<TYPES>,
        membership: &EpochMembershipCoordinator<TYPES>,
    ) {
        if let Err(err) = track_decided_qc_participation(
            leaf.justify_qc(),
            membership,
            &mut self.validator,
            &mut self.vote,
        ) {
            tracing::warn!(%err, "failed to update vote participation epoch");
        }
    }

    pub fn current_proposal_participation(&self) -> HashMap<TYPES::SignatureKey, f64> {
        self.validator.current_proposal_participation()
    }

    pub fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<TYPES::SignatureKey, f64> {
        self.validator.proposal_participation(epoch)
    }

    pub fn current_vote_participation(
        &self,
    ) -> HashMap<<TYPES::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.vote.current_vote_participation()
    }

    pub fn vote_participation(
        &self,
        epoch: EpochNumber,
    ) -> HashMap<<TYPES::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.vote.vote_participation(Some(epoch))
    }
}

#[derive(Debug, Clone)]
struct ValidatorParticipation<TYPES: NodeType> {
    epoch: EpochNumber,
    /// Current epoch participation by key maps key -> (num leader, num times proposed)
    current_epoch_participation: ValidatorParticipationMap<TYPES>,

    /// Previous epochs participation ordered by epoch number, maps epoch -> key -> (num leader, num times proposed)
    previous_epoch_participation: BTreeMap<EpochNumber, ValidatorParticipationMap<TYPES>>,
}

impl<TYPES: NodeType> ValidatorParticipation<TYPES> {
    fn new() -> Self {
        Self::new_in_epoch(EpochNumber::genesis())
    }

    fn new_in_epoch(epoch: EpochNumber) -> Self {
        Self {
            epoch,
            current_epoch_participation: HashMap::new(),
            previous_epoch_participation: BTreeMap::new(),
        }
    }

    fn update_participation(
        &mut self,
        key: TYPES::SignatureKey,
        epoch: EpochNumber,
        proposed: bool,
    ) {
        let participation = match epoch.cmp(&self.epoch) {
            std::cmp::Ordering::Greater => {
                self.update_participation_epoch(epoch);
                &mut self.current_epoch_participation
            },
            std::cmp::Ordering::Equal => &mut self.current_epoch_participation,
            std::cmp::Ordering::Less => {
                let Some(archived) = self.previous_epoch_participation.get_mut(&epoch) else {
                    return;
                };
                archived
            },
        };
        let entry = participation.entry(key).or_insert((0, 0));
        if proposed {
            entry.1 += 1;
        }
        entry.0 += 1;
    }

    fn update_participation_epoch(&mut self, epoch: EpochNumber) {
        if epoch <= self.epoch {
            return;
        }
        self.previous_epoch_participation
            .insert(self.epoch, self.current_epoch_participation.clone());

        self.previous_epoch_participation =
            self.previous_epoch_participation
                .split_off(&EpochNumber::new(
                    self.epoch.saturating_sub(EPOCH_PARTICIPATION_HISTORY),
                ));

        self.epoch = epoch;
        self.current_epoch_participation = HashMap::new();
    }

    fn current_proposal_participation(&self) -> HashMap<TYPES::SignatureKey, f64> {
        self.current_epoch_participation
            .iter()
            .map(|(key, (leader, proposed))| {
                (
                    key.clone(),
                    if *leader == 0 {
                        0.0
                    } else {
                        *proposed as f64 / *leader as f64
                    },
                )
            })
            .collect()
    }
    fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<TYPES::SignatureKey, f64> {
        let tracked_participation = if epoch == self.epoch {
            self.current_epoch_participation.clone()
        } else {
            self.previous_epoch_participation
                .get(&epoch)
                .unwrap_or(&HashMap::new())
                .clone()
        };

        tracked_participation
            .iter()
            .map(|(key, (leader, proposed))| {
                (
                    key.clone(),
                    if *leader == 0 {
                        0.0
                    } else {
                        *proposed as f64 / *leader as f64
                    },
                )
            })
            .collect()
    }

    fn current_epoch(&self) -> EpochNumber {
        self.epoch
    }
}

type VoteParticipationMap<TYPES> = (
    HashMap<<<TYPES as NodeType>::SignatureKey as SignatureKey>::VerificationKeyType, u64>,
    u64,
);

#[derive(Clone, Debug)]
struct VoteParticipation<TYPES: NodeType> {
    /// Current epoch
    epoch: Option<EpochNumber>,

    /// Current stake_table
    stake_table: HSStakeTable<TYPES>,

    /// Success threshold
    success_threshold: U256,

    /// Set of views in the current epoch
    view_set: HashSet<ViewNumber>,

    /// Number of views in the current epoch
    current_epoch_num_views: u64,

    /// Current epoch participation by key maps key -> num times voted
    current_epoch_participation:
        HashMap<<TYPES::SignatureKey as SignatureKey>::VerificationKeyType, u64>,

    /// Previous epochs participation ordered by epoch number, maps epoch -> key -> num times voted
    previous_epoch_participation: BTreeMap<Option<EpochNumber>, VoteParticipationMap<TYPES>>,
}

impl<TYPES: NodeType> VoteParticipation<TYPES> {
    fn new(
        stake_table: HSStakeTable<TYPES>,
        success_threshold: U256,
        epoch: Option<EpochNumber>,
    ) -> Self {
        let current_epoch_participation: HashMap<_, _> = stake_table
            .iter()
            .map({
                |peer_config| {
                    (
                        peer_config
                            .stake_table_entry
                            .public_key()
                            .to_verification_key(),
                        0u64,
                    )
                }
            })
            .collect();
        Self {
            epoch,
            stake_table,
            success_threshold,
            view_set: HashSet::new(),
            current_epoch_num_views: 0u64,
            current_epoch_participation,
            previous_epoch_participation: BTreeMap::new(),
        }
    }

    fn unresolved_stake_table() -> (HSStakeTable<TYPES>, U256) {
        (HSStakeTable::default(), U256::MAX)
    }

    fn stake_table_unresolved(&self) -> bool {
        self.stake_table.is_empty()
    }

    fn reseed_stake_table(&mut self, stake_table: HSStakeTable<TYPES>, threshold: U256) {
        self.current_epoch_participation = stake_table
            .iter()
            .map(|peer_config| {
                (
                    peer_config
                        .stake_table_entry
                        .public_key()
                        .to_verification_key(),
                    0u64,
                )
            })
            .collect();
        self.stake_table = stake_table;
        self.success_threshold = threshold;
    }

    fn update_participation(&mut self, qc: QuorumCertificate2<TYPES>) -> Result<()> {
        ensure!(
            qc.epoch() == self.epoch,
            info!(
                "Incorrect epoch while updating vote participation, current epoch: {:?}, QC epoch \
                 {:?}",
                self.epoch,
                qc.epoch()
            )
        );
        ensure!(
            !self.view_set.contains(&qc.view_number()),
            info!(
                "Participation for view {} already updated",
                qc.view_number()
            )
        );
        let signers = qc
            .signers(
                &StakeTableEntries::<TYPES>::from(self.stake_table.clone()).0,
                self.success_threshold,
            )
            .context(|e| warn!("Tracing signers: {e}"))?;
        for vk in signers {
            let Some(votes) = self.current_epoch_participation.get_mut(&vk) else {
                bail!(warn!(
                    "Trying to update vote participation for unknown key: {:?}",
                    vk
                ));
            };
            *votes += 1;
        }
        self.view_set.insert(qc.view_number());
        self.current_epoch_num_views += 1;
        Ok(())
    }

    fn update_participation_epoch(
        &mut self,
        stake_table: HSStakeTable<TYPES>,
        success_threshold: U256,
        epoch: Option<EpochNumber>,
    ) -> Result<()> {
        ensure!(
            epoch >= self.epoch,
            warn!(
                "New epoch less than current epoch while updating vote participation epoch, \
                 current epoch: {:?}, new epoch {:?}",
                self.epoch, epoch
            )
        );
        // Same epoch, do nothing
        if epoch == self.epoch {
            return Ok(());
        }

        self.previous_epoch_participation.insert(
            self.epoch,
            (
                self.current_epoch_participation.clone(),
                self.current_epoch_num_views,
            ),
        );

        self.previous_epoch_participation = self.previous_epoch_participation.split_off(
            &self
                .epoch
                .map(|e| EpochNumber::new(e.saturating_sub(EPOCH_PARTICIPATION_HISTORY))),
        );

        self.previous_epoch_participation.insert(
            self.epoch,
            (
                self.current_epoch_participation.clone(),
                self.current_epoch_num_views,
            ),
        );

        self.previous_epoch_participation = self.previous_epoch_participation.split_off(
            &self
                .epoch
                .map(|e| EpochNumber::new(e.saturating_sub(EPOCH_PARTICIPATION_HISTORY))),
        );

        self.epoch = epoch;
        self.current_epoch_num_views = 0;
        self.view_set = HashSet::new();
        let current_epoch_participation: HashMap<_, _> = stake_table
            .iter()
            .map({
                |peer_config| {
                    (
                        peer_config
                            .stake_table_entry
                            .public_key()
                            .to_verification_key(),
                        0u64,
                    )
                }
            })
            .collect();
        self.current_epoch_participation = current_epoch_participation;
        self.stake_table = stake_table;
        self.success_threshold = success_threshold;
        Ok(())
    }

    fn current_vote_participation(
        &self,
    ) -> HashMap<<TYPES::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.current_epoch_participation
            .iter()
            .map(|(key, votes)| {
                (
                    key.clone(),
                    Self::calculate_ratio(votes, self.current_epoch_num_views),
                )
            })
            .collect()
    }
    fn vote_participation(
        &self,
        epoch: Option<EpochNumber>,
    ) -> HashMap<<TYPES::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        let (participation, num_views) = if epoch == self.epoch {
            (
                self.current_epoch_participation.clone(),
                self.current_epoch_num_views,
            )
        } else {
            self.previous_epoch_participation
                .get(&epoch)
                .unwrap_or(&(HashMap::new(), 0))
                .clone()
        };

        if num_views == 0 {
            return HashMap::new();
        }

        participation
            .iter()
            .map(|(key, votes)| (key.clone(), Self::calculate_ratio(votes, num_views)))
            .collect()
    }

    fn calculate_ratio(num_votes: &u64, total_views: u64) -> f64 {
        if total_views == 0 {
            0.0
        } else {
            *num_votes as f64 / total_views as f64
        }
    }

    fn current_epoch(&self) -> Option<EpochNumber> {
        self.epoch
    }
}

fn resolve_participation_stake_table<TYPES: NodeType>(
    membership: &EpochMembershipCoordinator<TYPES>,
    epoch: Option<EpochNumber>,
) -> (HSStakeTable<TYPES>, U256) {
    match membership.stake_table_for_epoch(epoch) {
        Ok(m) => (
            HSStakeTable::from_iter(m.stake_table()),
            m.success_threshold(),
        ),
        Err(err) => {
            tracing::warn!(?epoch, %err, "no stake table for participation tracking");
            VoteParticipation::unresolved_stake_table()
        },
    }
}

fn track_decided_qc_participation<TYPES: NodeType>(
    qc: &QuorumCertificate2<TYPES>,
    membership: &EpochMembershipCoordinator<TYPES>,
    validator: &mut ValidatorParticipation<TYPES>,
    vote: &mut VoteParticipation<TYPES>,
) -> Result<()> {
    let qc_epoch = qc.epoch();
    if let Some(epoch) = qc_epoch
        && epoch > validator.current_epoch()
    {
        validator.update_participation_epoch(epoch);
    }
    if qc_epoch > vote.current_epoch() {
        let (stake_table, success_threshold) =
            resolve_participation_stake_table(membership, qc_epoch);
        vote.update_participation_epoch(stake_table, success_threshold, qc_epoch)
            .context(warn!("Updating vote participation"))?;
    } else if qc_epoch == vote.current_epoch()
        && vote.stake_table_unresolved()
        && let Ok(m) = membership.stake_table_for_epoch(qc_epoch)
    {
        vote.reseed_stake_table(
            HSStakeTable::from_iter(m.stake_table()),
            m.success_threshold(),
        );
    }
    if let Err(e) = vote.update_participation(qc.clone()) {
        tracing::warn!("Failed to update vote participation: {e}");
    }
    Ok(())
}

/// This struct holds a payload and its metadata
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PayloadWithMetadata<TYPES: NodeType> {
    pub payload: TYPES::BlockPayload,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

/// Contains several `ConsensusMetrics` that we're interested in from the consensus interfaces
#[derive(Clone, Debug)]
pub struct ConsensusMetricsValue {
    /// The number of last synced block height
    pub last_synced_block_height: Box<dyn Gauge>,
    /// The number of last decided view
    pub last_decided_view: Box<dyn Gauge>,
    /// The number of the last voted view
    pub last_voted_view: Box<dyn Gauge>,
    /// Number of timestamp for the last decided time
    pub last_decided_time: Box<dyn Gauge>,
    /// The current view
    pub current_view: Box<dyn Gauge>,
    /// Number of views that are in-flight since the last decided view
    pub number_of_views_since_last_decide: Box<dyn Gauge>,
    /// Number of views that are in-flight since the last anchor view
    pub number_of_views_per_decide_event: Box<dyn Histogram>,
    /// Duration of views as leader
    pub view_duration_as_leader: Box<dyn Histogram>,
    /// Number of invalid QCs we've seen since the last commit.
    pub invalid_qc: Box<dyn Gauge>,
    /// Number of outstanding transactions
    pub outstanding_transactions: Box<dyn Gauge>,
    /// Memory size in bytes of the serialized transactions still outstanding
    pub outstanding_transactions_memory_size: Box<dyn Gauge>,
    /// Number of views that timed out
    pub number_of_timeouts: Box<dyn Counter>,
    /// Number of views that timed out as leader
    pub number_of_timeouts_as_leader: Box<dyn Counter>,
    /// The number of empty blocks that have been proposed
    pub number_of_empty_blocks_proposed: Box<dyn Counter>,
    /// Number of events in the hotshot event queue
    pub internal_event_queue_len: Box<dyn Gauge>,
    /// Time from proposal creation to decide time
    pub proposal_to_decide_time: Box<dyn Histogram>,
    /// Time from proposal received to proposal creation
    pub previous_proposal_to_proposal_time: Box<dyn Histogram>,
    /// Finalized bytes per view
    pub finalized_bytes: Box<dyn Histogram>,
    /// The duration of the validate and apply header
    pub validate_and_apply_header_duration: Box<dyn Histogram>,
    /// The duration of update leaf
    pub update_leaf_duration: Box<dyn Histogram>,
    /// The time it took to calculate the disperse
    pub vid_disperse_duration: Box<dyn Histogram>,
    /// Run time of VID reconstruction attempts that concatenated the systematic shares
    pub vid_reconstruct_systematic_duration: Box<dyn Histogram>,
    /// Run time of VID reconstruction attempts that erasure-decoded a threshold of shares
    pub vid_reconstruct_decode_duration: Box<dyn Histogram>,
    /// VID reconstruction attempts that failed (bad shares or a non-codeword)
    pub vid_reconstruct_failures: Box<dyn Counter>,
    /// VID reconstruction attempts that lost the race to the other path
    pub vid_reconstruct_lost: Box<dyn Counter>,
    /// Payloads obtained by concatenating the systematic shares
    pub vid_payloads_systematic: Box<dyn Counter>,
    /// Payloads obtained by erasure-decoding a threshold of shares
    pub vid_payloads_decoded: Box<dyn Counter>,
    /// Payloads obtained whole from a peer
    pub vid_payloads_fetched: Box<dyn Counter>,
}

impl ConsensusMetricsValue {
    /// Create a new instance of this [`ConsensusMetricsValue`] struct, setting all the counters and gauges
    #[must_use]
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            last_synced_block_height: metrics
                .create_gauge(String::from("last_synced_block_height"), None),
            last_decided_view: metrics.create_gauge(String::from("last_decided_view"), None),
            last_voted_view: metrics.create_gauge(String::from("last_voted_view"), None),
            last_decided_time: metrics.create_gauge(String::from("last_decided_time"), None),
            current_view: metrics.create_gauge(String::from("current_view"), None),
            number_of_views_since_last_decide: metrics
                .create_gauge(String::from("number_of_views_since_last_decide"), None),
            number_of_views_per_decide_event: metrics
                .create_histogram(String::from("number_of_views_per_decide_event"), None),
            view_duration_as_leader: metrics
                .create_histogram(String::from("view_duration_as_leader"), None),
            invalid_qc: metrics.create_gauge(String::from("invalid_qc"), None),
            outstanding_transactions: metrics
                .create_gauge(String::from("outstanding_transactions"), None),
            outstanding_transactions_memory_size: metrics
                .create_gauge(String::from("outstanding_transactions_memory_size"), None),
            number_of_timeouts: metrics.create_counter(String::from("number_of_timeouts"), None),
            number_of_timeouts_as_leader: metrics
                .create_counter(String::from("number_of_timeouts_as_leader"), None),
            number_of_empty_blocks_proposed: metrics
                .create_counter(String::from("number_of_empty_blocks_proposed"), None),
            internal_event_queue_len: metrics
                .create_gauge(String::from("internal_event_queue_len"), None),
            proposal_to_decide_time: metrics
                .create_histogram(String::from("proposal_to_decide_time"), None),
            previous_proposal_to_proposal_time: metrics
                .create_histogram(String::from("previous_proposal_to_proposal_time"), None),
            finalized_bytes: metrics.create_histogram(String::from("finalized_bytes"), None),
            validate_and_apply_header_duration: metrics.create_histogram(
                String::from("validate_and_apply_header_duration"),
                Some("seconds".to_string()),
            ),
            update_leaf_duration: metrics.create_histogram(
                String::from("update_leaf_duration"),
                Some("seconds".to_string()),
            ),
            vid_disperse_duration: metrics.create_histogram(
                String::from("vid_disperse_duration"),
                Some("seconds".to_string()),
            ),
            vid_reconstruct_systematic_duration: metrics.create_histogram(
                String::from("vid_reconstruct_systematic_duration"),
                Some("seconds".to_string()),
            ),
            vid_reconstruct_decode_duration: metrics.create_histogram(
                String::from("vid_reconstruct_decode_duration"),
                Some("seconds".to_string()),
            ),
            vid_reconstruct_failures: metrics
                .create_counter(String::from("vid_reconstruct_failures"), None),
            vid_reconstruct_lost: metrics
                .create_counter(String::from("vid_reconstruct_lost"), None),
            vid_payloads_systematic: metrics
                .create_counter(String::from("vid_payloads_systematic"), None),
            vid_payloads_decoded: metrics
                .create_counter(String::from("vid_payloads_decoded"), None),
            vid_payloads_fetched: metrics
                .create_counter(String::from("vid_payloads_fetched"), None),
        }
    }
}

impl Default for ConsensusMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}
