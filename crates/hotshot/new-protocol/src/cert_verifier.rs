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
    /// Identifies the `Verifiable`, e.g. `ViewNumber` or `EpochNumber`.
    type Key: Copy + Ord + Hash + Display + Send + Sync + 'static;

    type Output: Send + 'static;

    fn key(&self) -> Option<Self::Key>;

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
    type Key = ViewNumber;
    type Output = Self;

    fn key(&self) -> Option<ViewNumber> {
        Some(self.view_number())
    }

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
    type Key = EpochNumber;
    type Output = EpochChangeMessage<T, Validated>;

    fn key(&self) -> Option<EpochNumber> {
        self.epoch()
    }

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

        self.tasks.spawn_blocking(key, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.check(&entries, threshold, &lock) {
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

type VerifyOutcome<O> = (ViewNumber, Option<ValidCert<O>>);

/// Verifies certificates off the main coordinator thread.
///
/// Unlike [`CertVerifier`], these certificates are keyed by sender key
/// instead of view/epoch, helping a lagging node jump to the frontier. While a
/// certificate is verified, subsequent requests are dropped which bounds each
/// peer to one verification at a time. Only one verification runs per view:
/// copies of an in-flight view's certificate from other senders are parked
/// and tried in turn only if the in-flight one proves invalid.
pub struct CertBySenderVerifier<T: NodeType, C: Verifiable<T>> {
    tasks: JoinMap<T::SignatureKey, VerifyOutcome<C::Output>>,
    in_flight: BTreeMap<ViewNumber, T::SignatureKey>,
    parked: BTreeMap<ViewNumber, HashMap<T::SignatureKey, C>>,
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
            in_flight: BTreeMap::new(),
            parked: BTreeMap::new(),
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
    /// Dropped if the sender's previous submission is still being verified;
    /// parked if the view is already being verified for another sender. If
    /// the epoch's membership isn't ready the item is held and its epoch
    /// returned so the caller can drive that epoch's catchup.
    pub fn verify(&mut self, sender: T::SignatureKey, cert: C) -> Option<EpochNumber> {
        let view = cert.view_number();

        if view < self.lower_bound || self.completed.contains(&view) {
            return None;
        }

        if let Some(in_flight_sender) = self.in_flight.get(&view) {
            if *in_flight_sender != sender {
                self.parked.entry(view).or_default().insert(sender, cert);
            }
            return None;
        }

        if self.tasks.contains_key(&sender) {
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

        self.in_flight.insert(view, sender.clone());
        self.tasks.spawn_blocking(sender, move || {
            let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
            let threshold = membership.success_threshold();
            match cert.check(&entries, threshold, &lock) {
                Ok(valid) => (view, Some(ValidCert::new(valid, epoch))),
                Err(err) => {
                    warn!(%view, %epoch, %err, cert = type_name::<C>(), "invalid certificate");
                    (view, None)
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
        self.parked.remove(&view);
        if let Some(sender) = self.in_flight.remove(&view) {
            self.tasks.abort(&sender);
        }
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
                (_, Ok((view, Some(cert)))) => {
                    self.in_flight.remove(&view);
                    self.parked.remove(&view);
                    if view >= self.lower_bound && self.completed.insert(view) {
                        return Some(cert);
                    }
                },
                (_, Ok((view, None))) => {
                    self.invalid_certs += 1;
                    self.in_flight.remove(&view);
                    self.promote_parked(view);
                },
                (sender, Err(err)) => {
                    if err.is_panic() {
                        error!(?sender, %err, cert = type_name::<C>(), "cert verification task panic");
                    }
                    // Don't strand parked copies of the failed sender's view.
                    let view = self
                        .in_flight
                        .iter()
                        .find_map(|(view, s)| (*s == sender).then_some(*view));
                    if let Some(view) = view {
                        self.in_flight.remove(&view);
                        self.promote_parked(view);
                    }
                },
            }
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.completed = self.completed.split_off(&view);
        self.parked = self.parked.split_off(&view);
        self.pending.retain(|_, c| c.view_number() >= view);
        let live = self.in_flight.split_off(&view);
        for sender in self.in_flight.values() {
            self.tasks.abort(sender);
        }
        self.in_flight = live;
        self.lower_bound = view;
    }

    pub fn num_invalid_certs(&self) -> u64 {
        self.invalid_certs
    }

    /// Try `view`'s parked copies until one spawns.
    fn promote_parked(&mut self, view: ViewNumber) {
        if view < self.lower_bound || self.completed.contains(&view) {
            self.parked.remove(&view);
            return;
        }
        while let Some((sender, cert)) = self.next_parked_sender(view) {
            self.verify(sender, cert);
            if self.in_flight.contains_key(&view) {
                return;
            }
        }
    }

    fn next_parked_sender(&mut self, view: ViewNumber) -> Option<(T::SignatureKey, C)> {
        let map = self.parked.get_mut(&view)?;
        let sender = map.keys().next().cloned()?;
        let cert = map.remove(&sender)?;
        if map.is_empty() {
            self.parked.remove(&view);
        }
        Some((sender, cert))
    }
}

/// The coordinator's network-certificate verifiers, one per certificate type.
pub struct CertVerifiers<T: NodeType> {
    pub cert1: CertVerifier<T, Certificate1<T>>,
    pub cert2: CertVerifier<T, Certificate2<T>>,
    pub timeout: CertBySenderVerifier<T, TimeoutCertificate2<T>>,
    pub advance: CertBySenderVerifier<T, Certificate1<T>>,
    pub epoch_change: CertVerifier<T, EpochChangeMessage<T, Unchecked>>,
}

impl<T: NodeType> CertVerifiers<T> {
    pub fn new(membership: EpochMembershipCoordinator<T>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            cert1: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            cert2: CertVerifier::new(membership.clone(), upgrade_lock.clone()),
            timeout: CertBySenderVerifier::new(membership.clone(), upgrade_lock.clone()),
            advance: CertBySenderVerifier::new(membership.clone(), upgrade_lock.clone()),
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
        self.advance.gc(view);
        self.epoch_change.gc(epoch);
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

#[cfg(test)]
mod tests {
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        simple_certificate::TimeoutCertificate2,
        traits::signature_key::SignatureKey,
        vote::HasViewNumber,
    };

    use super::CertBySenderVerifier;
    use crate::{
        helpers::test_upgrade_lock,
        tests::common::utils::{build_timeout_cert, mock_membership},
    };

    fn sender(i: u64) -> BLSPubKey {
        BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0
    }

    fn verifier() -> CertBySenderVerifier<TestTypes, TimeoutCertificate2<TestTypes>> {
        CertBySenderVerifier::new(mock_membership(), test_upgrade_lock())
    }

    fn valid_tc(view: u64) -> TimeoutCertificate2<TestTypes> {
        let epoch = EpochNumber::genesis();
        let membership = mock_membership().membership_for_epoch(Some(epoch)).unwrap();
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
        build_timeout_cert(
            ViewNumber::new(view),
            epoch,
            &membership,
            &pub_key,
            &priv_key,
        )
    }

    /// A certificate whose aggregate signature doesn't match its data.
    fn invalid_tc(view: u64) -> TimeoutCertificate2<TestTypes> {
        let mut tc = valid_tc(view);
        tc.signatures = valid_tc(view + 1).signatures;
        tc
    }

    /// Same-view copies park instead of spawning redundant tasks.
    #[tokio::test]
    async fn test_same_view_copies_are_parked() {
        let mut verifier = verifier();
        let view = ViewNumber::new(1);
        let tc = valid_tc(1);

        verifier.verify(sender(1), tc.clone());
        verifier.verify(sender(2), tc.clone());
        verifier.verify(sender(3), tc);
        assert_eq!(verifier.tasks.len(), 1);
        assert_eq!(verifier.parked.get(&view).unwrap().len(), 2);

        let cert = verifier.next().await.expect("certificate should verify");
        assert_eq!(cert.view_number(), view);
        assert!(verifier.parked.is_empty());
        assert!(verifier.in_flight.is_empty());
        assert!(verifier.next().await.is_none());
    }

    /// A parked copy is verified when the in-flight one proves invalid.
    #[tokio::test]
    async fn test_parked_copy_promoted_after_invalid() {
        let mut verifier = verifier();
        let view = ViewNumber::new(1);

        verifier.verify(sender(1), invalid_tc(1));
        verifier.verify(sender(2), valid_tc(1));
        assert_eq!(verifier.tasks.len(), 1);

        let cert = verifier
            .next()
            .await
            .expect("parked copy should be verified");
        assert_eq!(cert.view_number(), view);
        assert_eq!(verifier.num_invalid_certs(), 1);
    }

    /// Certificates for distinct views still verify concurrently.
    #[tokio::test]
    async fn test_distinct_views_verify_concurrently() {
        let mut verifier = verifier();

        verifier.verify(sender(1), valid_tc(1));
        verifier.verify(sender(2), valid_tc(2));
        assert_eq!(verifier.tasks.len(), 2);

        let mut views = vec![
            verifier.next().await.unwrap().view_number(),
            verifier.next().await.unwrap().view_number(),
        ];
        views.sort();
        assert_eq!(views, vec![ViewNumber::new(1), ViewNumber::new(2)]);
    }

    /// `mark_completed` drops parked copies, the in-flight result and new
    /// submissions for the view.
    #[tokio::test]
    async fn test_mark_completed_clears_view() {
        let mut verifier = verifier();
        let view = ViewNumber::new(1);

        verifier.verify(sender(1), valid_tc(1));
        verifier.verify(sender(2), valid_tc(1));
        verifier.mark_completed(view);
        assert!(verifier.parked.is_empty());
        assert!(verifier.next().await.is_none());

        verifier.verify(sender(3), valid_tc(1));
        assert!(verifier.tasks.is_empty());
    }
}
