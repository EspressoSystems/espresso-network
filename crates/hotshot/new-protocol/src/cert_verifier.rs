use std::{
    any::type_name,
    collections::{BTreeMap, BTreeSet, HashMap},
    mem,
    ops::Deref,
};

use alloy::primitives::U256;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{
        Certificate1, Certificate2, SimpleCertificate, Threshold, TimeoutCertificate2,
    },
    simple_vote::{HasEpoch, Voteable},
    stake_table::StakeTableEntries,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace::Result;
use tokio_util::task::JoinMap;
use tracing::{error, warn};

use crate::message::{EpochChangeMessage, Unchecked, Validated};

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

pub trait Verifiable<T: NodeType>: HasViewNumber + HasEpoch + Sized {
    type Output: Send + 'static;

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<Self::Output>;
}

impl<T, D, V> Verifiable<T> for SimpleCertificate<T, D, V>
where
    T: NodeType,
    D: Voteable<T> + HasEpoch + 'static,
    V: Threshold<T>,
    Self: Certificate<T, D> + Send + 'static,
{
    type Output = Self;

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<Self> {
        self.is_valid_cert(stake_table, threshold, upgrade_lock)?;
        Ok(self)
    }
}

impl<T: NodeType> Verifiable<T> for EpochChangeMessage<T, Unchecked> {
    type Output = EpochChangeMessage<T, Validated>;

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<Self::Output> {
        self.cert1
            .is_valid_cert(stake_table, threshold, upgrade_lock)?;
        self.cert2
            .is_valid_cert(stake_table, threshold, upgrade_lock)?;
        Ok(self.into_validated())
    }
}

/// Verifies certificates off the main coordinator thread.
///
/// The threshold-signature check is slow (> 1ms), so running it inline would
/// stall the consensus loop. Each item's check runs in a `spawn_blocking`
/// task; `next()` yields only those that pass. An item whose epoch
/// membership isn't known yet is held in `pending` and retried on
/// [`Self::retry_pending`]. One verifier handles one [`Verifiable`] type,
/// keyed by view so duplicates are dropped.
pub struct CertByViewVerifier<T: NodeType, C: Verifiable<T>> {
    tasks: JoinMap<ViewNumber, Option<ValidCert<C::Output>>>,
    pending: BTreeMap<ViewNumber, C>,
    completed: BTreeSet<ViewNumber>,
    lower_bound: ViewNumber,
    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
    invalid_certs: u64,
}

impl<T: NodeType, C: Verifiable<T> + Send + 'static> CertByViewVerifier<T, C> {
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

    /// Submit an item received from the network for verification. If the
    /// epoch's membership isn't ready the item is held and its epoch returned
    /// so the caller can drive that epoch's catchup. Duplicates are dropped.
    pub fn verify(&mut self, cert: C) -> Option<EpochNumber> {
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
            match cert.check(&entries, threshold, &lock) {
                Ok(valid) => Some(ValidCert::new(valid, epoch)),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
                    None
                },
            }
        });

        None
    }

    /// Record that this view's item was completed by other means.
    ///
    /// This can happen locally from votes for example.
    pub fn mark_completed(&mut self, view: ViewNumber) {
        if view < self.lower_bound {
            return;
        }
        self.completed.insert(view);
        self.pending.remove(&view);
        self.tasks.abort(&view);
    }

    /// Re-attempt any items deferred because their epoch stake table wasn't
    /// available. Called when new epoch data arrives. Returns the epochs
    /// whose stake table is still missing so the caller can keep driving their
    /// catchup.
    pub fn retry_pending(&mut self) -> Vec<EpochNumber> {
        mem::take(&mut self.pending)
            .into_values()
            .filter_map(|cert| self.verify(cert))
            .collect()
    }

    pub async fn next(&mut self) -> Option<ValidCert<C::Output>> {
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

/// Verifies certificates off the main coordinator thread.
///
/// Unlike [`CertByViewVerifier`], these certificates are keyed by sender key
/// instead of view, helping a lagging node jump to the frontier. While a
/// certificate is verified, subsequent requests are dropped which bounds each
/// peer to one verification at a time.
pub struct CertBySenderVerifier<T: NodeType, C: Verifiable<T>> {
    tasks: JoinMap<T::SignatureKey, Option<ValidCert<C::Output>>>,
    pending: HashMap<T::SignatureKey, C>,
    completed: BTreeSet<ViewNumber>,
    lower_bound: ViewNumber,
    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
    invalid_certs: u64,
}

impl<T: NodeType, C: Verifiable<T> + Send + 'static> CertBySenderVerifier<T, C>
where
    C::Output: HasViewNumber,
{
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            tasks: JoinMap::new(),
            pending: HashMap::new(),
            completed: BTreeSet::new(),
            lower_bound: ViewNumber::genesis(),
            membership,
            upgrade_lock,
            invalid_certs: 0,
        }
    }

    /// Submit an item received from `sender` for verification.
    ///
    /// Dropped if the sender's previous submission is still being verified. If
    /// the epoch's membership isn't ready the item is held and its epoch
    /// returned so the caller can drive that epoch's catchup.
    pub fn verify(&mut self, sender: T::SignatureKey, cert: C) -> Option<EpochNumber> {
        let view = cert.view_number();

        if view < self.lower_bound
            || self.completed.contains(&view)
            || self.tasks.contains_key(&sender)
        {
            return None;
        }

        let Some(epoch) = cert.epoch() else {
            warn!(%view, cert = type_name::<C>(), "received certificate has no epoch number");
            return None;
        };

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            self.pending.insert(sender, cert);
            return Some(epoch);
        };

        let lock = self.upgrade_lock.clone();

        self.tasks.spawn_blocking(sender, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.check(&entries, threshold, &lock) {
                Ok(valid) => Some(ValidCert::new(valid, epoch)),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
                    None
                },
            }
        });

        None
    }

    /// Record that this view's item was completed by other means.
    ///
    /// This can happen locally from votes for example.
    pub fn mark_completed(&mut self, view: ViewNumber) {
        if view < self.lower_bound {
            return;
        }
        self.completed.insert(view);
        self.pending.retain(|_, c| c.view_number() != view);
    }

    /// Re-attempt any items deferred because their epoch stake table wasn't
    /// available. Returns the epochs whose stake table is still missing so
    /// the caller can keep driving their catchup.
    pub fn retry_pending(&mut self) -> Vec<EpochNumber> {
        mem::take(&mut self.pending)
            .into_iter()
            .filter_map(|(sender, cert)| self.verify(sender, cert))
            .collect()
    }

    pub async fn next(&mut self) -> Option<ValidCert<C::Output>> {
        loop {
            match self.tasks.join_next().await? {
                (_, Ok(Some(cert))) => {
                    let view = cert.view_number();
                    if view >= self.lower_bound && self.completed.insert(view) {
                        return Some(cert);
                    }
                },
                (_, Ok(None)) => {
                    self.invalid_certs += 1;
                },
                (sender, Err(err)) => {
                    if err.is_panic() {
                        error!(?sender, %err, cert = type_name::<C>(), "cert verification task panic");
                    }
                },
            }
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.completed = self.completed.split_off(&view);
        self.pending.retain(|_, c| c.view_number() >= view);
        self.lower_bound = view;
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.invalid_certs
    }
}

/// The coordinator's network-certificate verifiers, one per certificate type.
pub struct CertVerifiers<T: NodeType> {
    pub cert1: CertByViewVerifier<T, Certificate1<T>>,
    pub cert2: CertByViewVerifier<T, Certificate2<T>>,
    pub timeout: CertBySenderVerifier<T, TimeoutCertificate2<T>>,
    pub advance: CertBySenderVerifier<T, Certificate1<T>>,
    pub epoch_change: CertByViewVerifier<T, EpochChangeMessage<T, Unchecked>>,
}

impl<T: NodeType> CertVerifiers<T> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            cert1: CertByViewVerifier::new(membership.clone(), upgrade_lock.clone()),
            cert2: CertByViewVerifier::new(membership.clone(), upgrade_lock.clone()),
            timeout: CertBySenderVerifier::new(membership.clone(), upgrade_lock.clone()),
            advance: CertBySenderVerifier::new(membership.clone(), upgrade_lock.clone()),
            epoch_change: CertByViewVerifier::new(membership, upgrade_lock),
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
        for epoch in self.epoch_change.retry_pending() {
            request(epoch);
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.cert1.gc(view);
        self.cert2.gc(view);
        self.timeout.gc(view);
        self.advance.gc(view);
        self.epoch_change.gc(view);
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.cert1
            .num_invalid_certs()
            .saturating_add(self.cert2.num_invalid_certs())
            .saturating_add(self.timeout.num_invalid_certs())
            .saturating_add(self.advance.num_invalid_certs())
            .saturating_add(self.epoch_change.num_invalid_certs())
    }
}
