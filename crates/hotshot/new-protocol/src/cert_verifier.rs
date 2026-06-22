use std::{
    any::type_name,
    collections::{BTreeMap, BTreeSet},
    mem,
};

use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::TimeoutCertificate2,
    simple_vote::HasEpoch,
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::Certificate,
};
use tokio_util::task::JoinMap;
use tracing::{error, warn};

use crate::message::{Certificate1, Certificate2};

/// Verifies certificates received from the network off the main coordinator
/// thread.
///
/// The threshold-signature check is slow (> 1ms), so running it inline would
/// stall the consensus loop. Each certificate's check runs in a `spawn_blocking`
/// task; `next()` yields only those that pass. A certificate whose epoch
/// membership isn't known yet is held in `pending` and retried on
/// [`Self::retry_pending`]. One verifier handles one certificate type, keyed by
/// view so duplicates are dropped.
pub struct CertVerifier<T: NodeType, C> {
    /// In-flight verification tasks, keyed by the certificate's view.
    tasks: JoinMap<ViewNumber, Option<C>>,

    /// Certificates whose epoch stake table was not yet available. Retried when
    /// new epoch data arrives.
    pending: BTreeMap<ViewNumber, C>,

    /// Views that already produced a verified certificate.
    completed: BTreeSet<ViewNumber>,

    /// The GC threshold; certificates below this view are ignored.
    lower_bound: ViewNumber,

    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType, C: Send + 'static> CertVerifier<T, C> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            tasks: JoinMap::new(),
            pending: BTreeMap::new(),
            completed: BTreeSet::new(),
            lower_bound: ViewNumber::genesis(),
            membership,
            upgrade_lock,
        }
    }

    /// Submit a certificate received from the network for verification. If the
    /// epoch's membership isn't ready the cert is held and its epoch returned so
    /// the caller can drive that epoch's catchup. Duplicates are dropped.
    pub fn verify<A>(&mut self, cert: C) -> Option<EpochNumber>
    where
        C: Certificate<T, A> + HasEpoch,
    {
        let view = cert.view_number();
        if view < self.lower_bound
            || self.completed.contains(&view)
            || self.tasks.contains_key(&view)
            || self.pending.contains_key(&view)
        {
            return None;
        }
        let Some(epoch) = cert.epoch() else {
            warn!(%view, cert = type_name::<C>(), "received certificate has no epoch number");
            return None;
        };
        // Require the full randomized membership (stake table *and* DRB): a
        // verified cert lets consensus jump to its view, so a catching-up node
        // must have the epoch's DRB before it advances into that epoch.
        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            // Membership not ready; hold the cert and return its epoch so the
            // caller drives that epoch's catchup.
            self.pending.insert(view, cert);
            return Some(epoch);
        };
        let lock = self.upgrade_lock.clone();
        // Run the slow, CPU-bound signature check on the blocking pool.
        self.tasks.spawn_blocking(view, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.is_valid_cert(&entries, threshold, &lock) {
                Ok(()) => Some(cert),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "received certificate not verified");
                    None
                },
            }
        });
        None
    }

    /// Re-attempt any certificates deferred because their epoch stake table
    /// wasn't available. Called when new epoch data arrives. Returns the epochs
    /// whose stake table is still missing so the caller can keep driving their
    /// catchup.
    pub fn retry_pending<A>(&mut self) -> Vec<EpochNumber>
    where
        C: Certificate<T, A> + HasEpoch,
    {
        mem::take(&mut self.pending)
            .into_values()
            .filter_map(|cert| self.verify(cert))
            .collect()
    }

    /// Yield the next certificate that passed verification.
    pub async fn next(&mut self) -> Option<C> {
        loop {
            match self.tasks.join_next().await {
                Some((view, Ok(Some(cert)))) => {
                    if view >= self.lower_bound {
                        self.completed.insert(view);
                        return Some(cert);
                    }
                },
                // Invalid signature or stale view.
                Some((_, Ok(None))) => {},
                Some((view, Err(err))) => {
                    if !err.is_cancelled() {
                        error!(%view, %err, cert = type_name::<C>(), "cert verification task panic");
                    }
                },
                None => return None,
            }
        }
    }

    /// Drop bookkeeping below `view` and abort verification tasks for views we
    /// no longer care about.
    pub fn gc(&mut self, view: ViewNumber) {
        self.completed = self.completed.split_off(&view);
        self.pending = self.pending.split_off(&view);
        self.lower_bound = view;
        self.tasks.abort_matching(|v| *v < view);
    }
}

/// The coordinator's network-certificate verifiers, one per certificate type.
pub struct CertVerifiers<T: NodeType> {
    pub cert1: CertVerifier<T, Certificate1<T>>,
    pub cert2: CertVerifier<T, Certificate2<T>>,
    pub timeout: CertVerifier<T, TimeoutCertificate2<T>>,
}

impl<T: NodeType> CertVerifiers<T> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            cert1: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            cert2: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            timeout: CertVerifier::new(membership, upgrade_lock),
        }
    }

    /// Re-attempt every deferred certificate; returns the epochs still missing
    /// membership so the caller can keep driving their catchup.
    pub fn retry_pending(&mut self) -> Vec<EpochNumber> {
        let mut epochs = self.cert1.retry_pending();
        epochs.extend(self.cert2.retry_pending());
        epochs.extend(self.timeout.retry_pending());
        epochs
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.cert1.gc(view);
        self.cert2.gc(view);
        self.timeout.gc(view);
    }
}
