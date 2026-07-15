use std::{
    any::type_name,
    collections::{BTreeMap, BTreeSet},
    mem,
    ops::Deref,
};

use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate1, Certificate2, TimeoutCertificate2},
    simple_vote::HasEpoch,
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::{Certificate, HasViewNumber},
};
use tokio_util::task::JoinMap;
use tracing::{error, warn};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValidCert<C> {
    cert: C,
    epoch: EpochNumber,
}

impl<C> ValidCert<C> {
    pub(crate) fn new(cert: C, epoch: EpochNumber) -> Self {
        Self { cert, epoch }
    }

    pub fn cert(&self) -> &C {
        &self.cert
    }

    pub fn epoch(&self) -> EpochNumber {
        self.epoch
    }

    pub fn into_cert(self) -> C {
        self.cert
    }
}

impl<C> Deref for ValidCert<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.cert()
    }
}

impl<C: HasViewNumber> HasViewNumber for ValidCert<C> {
    fn view_number(&self) -> ViewNumber {
        self.cert.view_number()
    }
}

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
    tasks: JoinMap<ViewNumber, Option<ValidCert<C>>>,
    pending: BTreeMap<ViewNumber, C>,
    completed: BTreeSet<ViewNumber>,
    lower_bound: ViewNumber,
    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
    invalid_certs: u64,
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
            invalid_certs: 0,
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

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            self.pending.insert(view, cert);
            return Some(epoch);
        };

        let lock = self.upgrade_lock.clone();

        self.tasks.spawn_blocking(view, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.is_valid_cert(&entries, threshold, &lock) {
                Ok(()) => Some(ValidCert::new(cert, epoch)),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
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

    pub async fn next(&mut self) -> Option<ValidCert<C>> {
        loop {
            match self.tasks.join_next().await? {
                (view, Ok(Some(cert))) => {
                    if view >= self.lower_bound {
                        self.completed.insert(view);
                        return Some(cert);
                    }
                },
                (_, Ok(None)) => {
                    self.invalid_certs += 1;
                },
                (view, Err(err)) => {
                    if err.is_panic() {
                        error!(%view, %err, cert = type_name::<C>(), "cert verification task panic");
                    }
                },
            }
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.completed = self.completed.split_off(&view);
        self.pending = self.pending.split_off(&view);
        self.lower_bound = view;
        self.tasks.abort_matching(|v| *v < view);
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.invalid_certs
    }
}

/// The coordinator's network-certificate verifiers, one per certificate type.
pub struct CertVerifiers<T: NodeType> {
    pub cert1: CertVerifier<T, Certificate1<T>>,
    pub cert2: CertVerifier<T, Certificate2<T>>,
    pub timeout: CertVerifier<T, TimeoutCertificate2<T>>,
    pub advance: CertVerifier<T, Certificate1<T>>,
}

impl<T: NodeType> CertVerifiers<T> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            cert1: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            cert2: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            timeout: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            advance: CertVerifier::new(membership, upgrade_lock),
        }
    }

    pub fn retry_pending<F>(&mut self, mut request: F)
    where
        F: FnMut(EpochNumber),
    {
        for epoch in self.cert1.retry_pending() {
            request(epoch);
        }
        for epoch in self.cert2.retry_pending() {
            request(epoch);
        }
        for epoch in self.timeout.retry_pending() {
            request(epoch);
        }
        for epoch in self.advance.retry_pending() {
            request(epoch);
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.cert1.gc(view);
        self.cert2.gc(view);
        self.timeout.gc(view);
        self.advance.gc(view);
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.cert1
            .num_invalid_certs()
            .saturating_add(self.cert2.num_invalid_certs())
            .saturating_add(self.timeout.num_invalid_certs())
            .saturating_add(self.advance.num_invalid_certs())
    }
}
