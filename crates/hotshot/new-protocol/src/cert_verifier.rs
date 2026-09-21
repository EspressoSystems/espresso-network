use std::{
    any::type_name,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Display,
    hash::Hash,
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
        TimeoutCertificate3,
    },
    simple_vote::{HasEpoch, Voteable},
    stake_table::StakeTableEntries,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace::{Result, Wrap};
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

    pub fn map<D, F>(self, f: F) -> ValidCert<D>
    where
        F: FnOnce(C) -> D,
    {
        ValidCert {
            cert: f(self.cert),
            epoch: self.epoch,
        }
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
    /// Identifies the `Verifiable`, e.g. `ViewNumber` or `EpochNumber`.
    type Key: Copy + Ord + Hash + Display + Send + Sync + 'static;

    type Output: Send + 'static;

    fn key(&self) -> Option<Self::Key>;

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        epoch_height: u64,
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
    type Key = ViewNumber;
    type Output = Self;

    fn key(&self) -> Option<ViewNumber> {
        Some(self.view_number())
    }

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        _epoch_height: u64,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<Self> {
        self.is_valid_cert(stake_table, threshold, upgrade_lock)?;
        Ok(self)
    }
}

impl<T: NodeType> Verifiable<T> for EpochChangeMessage<T, Unchecked> {
    type Key = EpochNumber;
    type Output = EpochChangeMessage<T, Validated>;

    fn key(&self) -> Option<EpochNumber> {
        self.epoch()
    }

    fn check(
        self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        epoch_height: u64,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<Self::Output> {
        self.well_formed(epoch_height).wrap()?;
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
/// membership isn't known yet is held in `pending_membership` and retried on
/// [`Self::retry_pending`].
///
/// Items are deduplicated per ([`Verifiable::Key`], sender) and verified one
/// at a time per key, trying the next sender's item if one proves invalid; a
/// faulty sender can neither shadow a key nor hold more than one slot per
/// key. Since intake bounds the key space, memory is bounded by (admissible
/// keys * committee size).
pub struct CertVerifier<T: NodeType, C: Verifiable<T>> {
    tasks: JoinMap<C::Key, Option<ValidCert<C::Output>>>,
    pending_task: BTreeMap<C::Key, HashMap<T::SignatureKey, C>>,
    pending_membership: BTreeMap<C::Key, HashMap<T::SignatureKey, C>>,
    completed: BTreeSet<C::Key>,
    lower_bound: Option<C::Key>,
    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
    invalid_certs: u64,
}

impl<T: NodeType, C: Verifiable<T> + Send + 'static> CertVerifier<T, C> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            tasks: JoinMap::new(),
            pending_task: BTreeMap::new(),
            pending_membership: BTreeMap::new(),
            completed: BTreeSet::new(),
            lower_bound: None,
            membership,
            upgrade_lock,
            invalid_certs: 0,
        }
    }

    /// Submit an item received from the network for verification. If the
    /// epoch's membership isn't ready the item is held and its epoch returned
    /// so the caller can drive that epoch's catchup. Duplicates are dropped.
    pub fn verify(&mut self, sender: T::SignatureKey, cert: C) -> Option<EpochNumber> {
        let Some(key) = cert.key() else {
            warn!(cert = type_name::<C>(), "certificate has no key");
            return None;
        };

        let Some(epoch) = cert.epoch() else {
            warn!(%key, cert = type_name::<C>(), "certificate has no epoch number");
            return None;
        };

        if self.is_stale(key) || self.completed.contains(&key) {
            return None;
        }

        if let Some(senders) = self.pending_task.get(&key)
            && senders.contains_key(&sender)
        {
            return None;
        }

        if let Some(senders) = self.pending_membership.get(&key)
            && senders.contains_key(&sender)
        {
            return None;
        }

        if self.tasks.contains_key(&key) {
            self.pending_task
                .entry(key)
                .or_default()
                .insert(sender, cert);
            return None;
        }

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            self.pending_membership
                .entry(key)
                .or_default()
                .insert(sender, cert);
            return Some(epoch);
        };

        let lock = self.upgrade_lock.clone();
        let epoch_height = *self.membership.epoch_height();

        self.tasks.spawn_blocking(key, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.check(&entries, threshold, epoch_height, &lock) {
                Ok(valid) => Some(ValidCert::new(valid, epoch)),
                Err(err) => {
                    warn!(%key, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
                    None
                },
            }
        });

        None
    }

    /// Record that this key's item was completed by other means.
    ///
    /// This can happen locally from votes for example.
    pub fn mark_completed(&mut self, key: C::Key) {
        if self.is_stale(key) {
            return;
        }
        self.completed.insert(key);
        self.pending_task.remove(&key);
        self.pending_membership.remove(&key);
        self.tasks.abort(&key);
    }

    /// Re-attempt any items deferred because their epoch stake table wasn't
    /// available. Called when new epoch data arrives. Returns the epochs
    /// whose stake table is still missing so the caller can keep driving their
    /// catchup.
    pub fn retry_pending(&mut self) -> Vec<EpochNumber> {
        mem::take(&mut self.pending_membership)
            .into_values()
            .flatten()
            .filter_map(|(sender, cert)| self.verify(sender, cert))
            .collect()
    }

    pub async fn next(&mut self) -> Option<ValidCert<C::Output>> {
        loop {
            match self.tasks.join_next().await? {
                (key, Ok(Some(cert))) => {
                    if !self.is_stale(key) {
                        self.completed.insert(key);
                        self.pending_task.remove(&key);
                        self.pending_membership.remove(&key);
                        return Some(cert);
                    }
                },
                (key, Ok(None)) => {
                    self.invalid_certs += 1;
                    if !self.is_stale(key)
                        && let Some((sender, cert)) = self.next_pending_sender(key)
                    {
                        self.verify(sender, cert);
                    }
                },
                (key, Err(err)) => {
                    if err.is_panic() {
                        error!(%key, %err, cert = type_name::<C>(), "cert verification task panic");
                    }
                    if !self.is_stale(key)
                        && let Some((sender, cert)) = self.next_pending_sender(key)
                    {
                        self.verify(sender, cert);
                    }
                },
            }
        }
    }

    pub fn gc(&mut self, key: C::Key) {
        self.completed = self.completed.split_off(&key);
        self.pending_task = self.pending_task.split_off(&key);
        self.pending_membership = self.pending_membership.split_off(&key);
        self.lower_bound = Some(key);
        self.tasks.abort_matching(|k| *k < key);
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.invalid_certs
    }

    fn next_pending_sender(&mut self, k: C::Key) -> Option<(T::SignatureKey, C)> {
        let map = self.pending_task.get_mut(&k)?;
        let sender = map.keys().next().cloned()?;
        let cert = map.remove(&sender)?;
        if map.is_empty() {
            self.pending_task.remove(&k);
        }
        Some((sender, cert))
    }

    fn is_stale(&self, key: C::Key) -> bool {
        self.lower_bound.is_some_and(|lb| key < lb)
    }
}

/// What a completed certificate retires: its view, or its view and epoch.
#[derive(Clone, Copy, Debug)]
pub enum Completion {
    PerView,
    PerViewAndEpoch,
}

/// Verifies certificates off the main coordinator thread.
///
/// Unlike [`CertVerifier`], these certificates are keyed by sender key
/// instead of view/epoch, helping a lagging node jump to the frontier. While a
/// certificate is verified, subsequent requests are dropped which bounds each
/// peer to one verification at a time.
pub struct CertBySenderVerifier<T: NodeType, C: Verifiable<T>> {
    tasks: JoinMap<T::SignatureKey, Option<ValidCert<C::Output>>>,
    pending: HashMap<T::SignatureKey, C>,
    completed: BTreeSet<(ViewNumber, EpochNumber)>,
    completion: Completion,
    lower_bound: ViewNumber,
    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
    invalid_certs: u64,
}

impl<T: NodeType, C: Verifiable<T> + Send + 'static> CertBySenderVerifier<T, C>
where
    C::Output: HasViewNumber,
{
    pub fn new(
        membership: EpochMembershipCoordinator<T>,
        upgrade_lock: UpgradeLock<T>,
        completion: Completion,
    ) -> Self {
        Self {
            tasks: JoinMap::new(),
            pending: HashMap::new(),
            completed: BTreeSet::new(),
            completion,
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

        if view < self.lower_bound || self.tasks.contains_key(&sender) {
            return None;
        }

        let Some(epoch) = cert.epoch() else {
            warn!(%view, cert = type_name::<C>(), "received certificate has no epoch number");
            return None;
        };

        if self.completed.contains(&self.completion_key(view, epoch)) {
            return None;
        }

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            self.pending.insert(sender, cert);
            return Some(epoch);
        };

        let lock = self.upgrade_lock.clone();
        let epoch_height = *self.membership.epoch_height();

        self.tasks.spawn_blocking(sender, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.check(&entries, threshold, epoch_height, &lock) {
                Ok(valid) => Some(ValidCert::new(valid, epoch)),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
                    None
                },
            }
        });

        None
    }

    /// Record that this view's item was completed by other means, in `epoch`.
    ///
    /// This can happen locally from votes for example.
    pub fn mark_completed(&mut self, view: ViewNumber, epoch: EpochNumber) {
        if view < self.lower_bound {
            return;
        }
        self.completed.insert(self.completion_key(view, epoch));
        let and_epoch = matches!(self.completion, Completion::PerViewAndEpoch);
        self.pending
            .retain(|_, c| c.view_number() != view || (and_epoch && c.epoch() != Some(epoch)));
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
                    let key = self.completion_key(view, cert.epoch());
                    if view >= self.lower_bound && self.completed.insert(key) {
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
        self.completed = self.completed.split_off(&(view, EpochNumber::new(0)));
        self.pending.retain(|_, c| c.view_number() >= view);
        self.lower_bound = view;
    }

    /// What a completed item is remembered by.
    fn completion_key(&self, view: ViewNumber, epoch: EpochNumber) -> (ViewNumber, EpochNumber) {
        match self.completion {
            Completion::PerView => (view, EpochNumber::new(0)),
            Completion::PerViewAndEpoch => (view, epoch),
        }
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.invalid_certs
    }
}

/// The coordinator's network-certificate verifiers, one per certificate type.
pub struct CertVerifiers<T: NodeType> {
    pub cert1: CertVerifier<T, Certificate1<T>>,
    pub cert2: CertVerifier<T, Certificate2<T>>,
    pub timeout: CertBySenderVerifier<T, TimeoutCertificate2<T>>,
    pub timeout3: CertBySenderVerifier<T, TimeoutCertificate3<T>>,
    pub advance: CertBySenderVerifier<T, Certificate1<T>>,
    pub epoch_change: CertVerifier<T, EpochChangeMessage<T, Unchecked>>,
}

impl<T: NodeType> CertVerifiers<T> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            cert1: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            cert2: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            timeout: CertBySenderVerifier::new(
                membership.clone(),
                upgrade_lock.clone(),
                Completion::PerView,
            ),
            timeout3: CertBySenderVerifier::new(
                membership.clone(),
                upgrade_lock.clone(),
                Completion::PerViewAndEpoch,
            ),
            advance: CertBySenderVerifier::new(
                membership.clone(),
                upgrade_lock.clone(),
                Completion::PerViewAndEpoch,
            ),
            epoch_change: CertVerifier::new(membership, upgrade_lock),
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
        for epoch in self.timeout3.retry_pending() {
            request(epoch);
        }
        for epoch in self.advance.retry_pending() {
            request(epoch);
        }
        for epoch in self.epoch_change.retry_pending() {
            request(epoch);
        }
    }

    pub fn gc(&mut self, view: ViewNumber, epoch: EpochNumber) {
        self.cert1.gc(view);
        self.cert2.gc(view);
        self.timeout.gc(view);
        self.timeout3.gc(view);
        self.advance.gc(view);
        self.epoch_change.gc(epoch);
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.cert1
            .num_invalid_certs()
            .saturating_add(self.cert2.num_invalid_certs())
            .saturating_add(self.timeout.num_invalid_certs())
            .saturating_add(self.timeout3.num_invalid_certs())
            .saturating_add(self.advance.num_invalid_certs())
            .saturating_add(self.epoch_change.num_invalid_certs())
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use committable::Committable;
    use hotshot::types::{BLSPubKey, SignatureKey as _};
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        simple_certificate::{TimeoutCertificate2, TimeoutCertificate3},
        simple_vote::{TimeoutData2, TimeoutData3},
    };

    use super::{CertBySenderVerifier, Completion};
    use crate::{
        helpers::{test_timeout_epoch_lock, test_upgrade_lock},
        tests::common::utils::mock_membership,
    };

    /// An unsigned certificate, which verification is bound to reject: what is
    /// under test is whether it is examined at all.
    fn junk_tc(view: ViewNumber, epoch: EpochNumber) -> TimeoutCertificate2<TestTypes> {
        let data = TimeoutData2 {
            view,
            epoch: Some(epoch),
        };
        TimeoutCertificate2::new(data.clone(), data.commit(), view, None, PhantomData)
    }

    fn junk_tc3(view: ViewNumber, epoch: EpochNumber) -> TimeoutCertificate3<TestTypes> {
        let data = TimeoutData3 { view, epoch };
        TimeoutCertificate3::new(data.clone(), data.commit(), view, None, PhantomData)
    }

    /// Completing a view in one epoch must not retire it in another, where the
    /// epoch is bound.
    ///
    /// A view at an epoch boundary can have a certificate from each committee.
    /// Retiring the view on the first would leave the second permanently
    /// unverified, with the one the node ends up acting on decided by arrival
    /// order.
    #[tokio::test]
    async fn completing_one_epoch_leaves_the_other_open() {
        let mut verifier = CertBySenderVerifier::<TestTypes, TimeoutCertificate3<TestTypes>>::new(
            mock_membership(),
            test_timeout_epoch_lock(),
            Completion::PerViewAndEpoch,
        );
        let view = ViewNumber::new(1);
        let (old, new) = (EpochNumber::genesis(), EpochNumber::genesis() + 1);
        let sender = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;

        verifier.mark_completed(view, old);

        // The same view under the other committee is still examined, and
        // rejected on its merits rather than dropped on the view alone.
        assert!(verifier.verify(sender, junk_tc3(view, new)).is_none());
        assert!(verifier.next().await.is_none());
        assert_eq!(verifier.num_invalid_certs(), 1);

        // The epoch that was marked stays retired, so nothing examines it.
        assert!(verifier.verify(sender, junk_tc3(view, old)).is_none());
        assert!(verifier.next().await.is_none());
        assert_eq!(verifier.num_invalid_certs(), 1);
    }

    /// Completing a view retires it under every epoch, where the epoch is not
    /// bound.
    ///
    /// The label is not covered by the signers, so a copy of the same
    /// certificate under another epoch is the same object, and verifying it
    /// again buys nothing: a second certificate for a view is dropped by
    /// `Consensus::handle_timeout_certificate` whatever it names.
    #[tokio::test]
    async fn completing_a_view_retires_every_epoch_of_it() {
        let mut verifier = CertBySenderVerifier::<TestTypes, TimeoutCertificate2<TestTypes>>::new(
            mock_membership(),
            test_upgrade_lock(),
            Completion::PerView,
        );
        let view = ViewNumber::new(1);
        let (ours, theirs) = (EpochNumber::genesis(), EpochNumber::genesis() + 1);
        let sender = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;

        verifier.mark_completed(view, ours);

        for epoch in [ours, theirs] {
            assert!(verifier.verify(sender, junk_tc(view, epoch)).is_none());
            assert!(verifier.next().await.is_none());
            assert_eq!(
                verifier.num_invalid_certs(),
                0,
                "a retired view must not be examined again under {epoch}"
            );
        }

        // A later view is untouched.
        assert!(verifier.verify(sender, junk_tc(view + 1, theirs)).is_none());
        assert!(verifier.next().await.is_none());
        assert_eq!(verifier.num_invalid_certs(), 1);
    }
}
