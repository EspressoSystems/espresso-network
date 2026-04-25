use std::collections::{BTreeMap, BTreeSet, HashMap};

use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_certificate::{LightClientStateUpdateCertificateV2, QuorumCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::{
        Certificate, HasViewNumber, LightClientStateUpdateVoteAccumulator, Vote, VoteAccumulator,
    },
};
use tokio::{
    sync::mpsc::{self},
    task::{AbortHandle, JoinSet},
};
use tracing::{debug, instrument, warn};

use crate::message::Vote1;

/// Combined collector for epoch-root views. Runs both a quorum-vote accumulator
/// (producing a `QuorumCertificate2`) and a light-client-state-update-vote
/// accumulator (producing a `LightClientStateUpdateCertificateV2`) against the
/// same `Vote1` stream, emitting the pair only when **both** cross threshold.
///
/// This preserves the old protocol's atomicity property: `Consensus` never sees
/// an epoch-root Cert1 without the matching `state_cert`.
pub struct EpochRootVoteCollector<T: NodeType> {
    per_view: BTreeMap<ViewNumber, (mpsc::Sender<Vote1<T>>, AbortHandle)>,
    completed: BTreeSet<ViewNumber>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    membership_cache: BTreeMap<EpochNumber, EpochMembership<T>>,
    upgrade_lock: UpgradeLock<T>,
    tasks: JoinSet<(
        QuorumCertificate2<T>,
        LightClientStateUpdateCertificateV2<T>,
    )>,
}

impl<T: NodeType> EpochRootVoteCollector<T> {
    #[instrument(level = "debug", skip_all)]
    pub fn new(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            per_view: BTreeMap::new(),
            completed: BTreeSet::new(),
            epoch_membership_coordinator,
            membership_cache: BTreeMap::new(),
            upgrade_lock,
            tasks: JoinSet::new(),
        }
    }

    pub async fn next(
        &mut self,
    ) -> Option<(
        QuorumCertificate2<T>,
        LightClientStateUpdateCertificateV2<T>,
    )> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok((cert1, state_cert))) => {
                    let view = cert1.view_number();
                    if self.completed.contains(&view) {
                        continue;
                    }
                    self.completed.insert(view);
                    return Some((cert1, state_cert));
                },
                Some(Err(e)) if e.is_cancelled() => {
                    debug!("Epoch-root vote collection task cancelled: {e}");
                },
                Some(Err(e)) => {
                    warn!("Error in epoch-root vote collection task: {e}");
                },
                None => return None,
            }
        }
    }

    /// Accumulate a `Vote1` for an epoch-root view. Caller must have verified
    /// `vote1.state_vote.is_some()` — this method panics otherwise, because the
    /// atomicity invariant is broken if it isn't true.
    pub async fn accumulate(&mut self, vote1: Vote1<T>) {
        debug_assert!(
            vote1.state_vote.is_some(),
            "EpochRootVoteCollector::accumulate called with a Vote1 missing state_vote"
        );
        let view = vote1.vote.view_number();
        if self.completed.contains(&view) {
            return;
        }
        let Some(membership) = self.resolve_membership(&vote1.vote).await else {
            return;
        };
        let (tx, _abort_handle) = self.per_view.entry(view).or_insert_with(|| {
            let (tx, rx) = mpsc::channel(100);
            let abort_handle = self.tasks.spawn(Self::run_per_view(
                rx,
                membership,
                self.upgrade_lock.clone(),
            ));
            (tx, abort_handle)
        });
        let _ = tx.send(vote1).await;
    }

    async fn resolve_membership(&mut self, vote: &QuorumVote2<T>) -> Option<EpochMembership<T>> {
        let epoch = vote.epoch()?;
        if let Some(m) = self.membership_cache.get(&epoch) {
            return Some(m.clone());
        }
        let m = self
            .epoch_membership_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .ok()?;
        self.membership_cache.insert(epoch, m.clone());
        Some(m)
    }

    #[instrument(level = "debug", skip_all)]
    async fn run_per_view(
        mut rx: mpsc::Receiver<Vote1<T>>,
        membership: EpochMembership<T>,
        lock: UpgradeLock<T>,
    ) -> (
        QuorumCertificate2<T>,
        LightClientStateUpdateCertificateV2<T>,
    ) {
        let mut quorum_accumulator =
            VoteAccumulator::<T, QuorumVote2<T>, QuorumCertificate2<T>>::new(lock.clone());
        let mut state_accumulator = LightClientStateUpdateVoteAccumulator::<T> {
            vote_outcomes: HashMap::new(),
            upgrade_lock: lock.clone(),
        };

        let mut quorum_cert: Option<QuorumCertificate2<T>> = None;
        let mut state_cert: Option<LightClientStateUpdateCertificateV2<T>> = None;
        let mut quorum_votes: Vec<QuorumVote2<T>> = Vec::new();

        while let Some(vote1) = rx.recv().await {
            let state_vote = match vote1.state_vote.clone() {
                Some(sv) => sv,
                None => continue, // defensive; coordinator filters this out
            };
            let bls_key = vote1.vote.signing_key();

            if quorum_cert.is_none() {
                match quorum_accumulator
                    .accumulate(&vote1.vote, membership.clone())
                    .await
                {
                    Some(cert) => {
                        let stake_table =
                            <QuorumCertificate2<T> as Certificate<T, _>>::stake_table(&membership)
                                .await;
                        let threshold =
                            <QuorumCertificate2<T> as Certificate<T, _>>::threshold(&membership)
                                .await;
                        match cert.is_valid_cert(
                            &StakeTableEntries::<T>::from(stake_table).0,
                            threshold,
                            &lock,
                        ) {
                            Ok(()) => {
                                quorum_cert = Some(cert);
                            },
                            Err(e) => {
                                warn!("Invalid quorum certificate formed at epoch-root view: {e}");
                                // Retry from previously-seen votes (mirror VoteCollector recovery).
                                quorum_votes.push(vote1.vote.clone());
                                quorum_votes.retain(|v| {
                                    let vote_commitment = generate_vote_commitment(v, &lock);
                                    vote_commitment.is_some_and(|commitment| {
                                        v.signing_key()
                                            .validate(&v.signature(), commitment.as_ref())
                                    })
                                });
                                quorum_accumulator = VoteAccumulator::new(lock.clone());
                                for v in &quorum_votes {
                                    if let Some(cert) =
                                        quorum_accumulator.accumulate(v, membership.clone()).await
                                    {
                                        quorum_cert = Some(cert);
                                        break;
                                    }
                                }
                            },
                        }
                    },
                    None => {
                        quorum_votes.push(vote1.vote.clone());
                    },
                }
            }

            if state_cert.is_none()
                && let Some(cert) = state_accumulator
                    .accumulate(&bls_key, &state_vote, &membership)
                    .await
            {
                state_cert = Some(cert);
            }

            if let (Some(q), Some(s)) = (&quorum_cert, &state_cert) {
                return (q.clone(), s.clone());
            }
        }
        // Channel closed without both certs forming; this task is effectively dead.
        // Await never returns — GC aborts via AbortHandle.
        futures::future::pending::<()>().await;
        unreachable!()
    }

    pub fn gc(&mut self, view: ViewNumber, epoch: EpochNumber) {
        let keep = self.per_view.split_off(&view);
        self.completed = self.completed.split_off(&view);
        for (_, handle) in self.per_view.values_mut() {
            handle.abort();
        }
        self.per_view = keep;
        self.membership_cache = self.membership_cache.split_off(&epoch);
    }
}

fn generate_vote_commitment<T: NodeType, V: Vote<T>>(
    vote: &V,
    upgrade_lock: &UpgradeLock<T>,
) -> Option<committable::Commitment<VersionedVoteData<T, V::Commitment>>> {
    match VersionedVoteData::new(vote.date().clone(), vote.view_number(), upgrade_lock) {
        Ok(data) => Some(data.commit()),
        Err(e) => {
            tracing::warn!("Failed to generate versioned vote data: {e}");
            None
        },
    }
}
