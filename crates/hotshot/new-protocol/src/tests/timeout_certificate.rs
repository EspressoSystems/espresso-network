//! The structural checks of [`TimeoutCertificate3::check_signatures`].
//!
//! Every certificate here is one [`TimeoutCertificate3::assemble`] builds with
//! a single change, so each test fails only the check it is named for. The
//! certificate has three groups: signers locked on nothing, signers locked on
//! view 2's `Certificate1`, and signers locked on view 4's, the latest.

use alloy::primitives::U256;
use committable::RawCommitmentBuilder;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembership,
    signature_key::BLSPubKey,
    simple_certificate::TimeoutCertificate3,
    simple_vote::{LockView, SimpleVote, TimeoutData3},
    stake_table::StakeTableEntries,
    traits::signature_key::{SignatureKey, StakeTableEntryType},
    vote::Vote,
};

use super::common::utils::{build_cert1, mock_membership};
use crate::{helpers::test_timeout_epoch_lock, message::Certificate1};

const VIEW: u64 = 10;

/// The signers of each group, by test key index.
const UNLOCKED: [u64; 3] = [0, 1, 2];
const LOCKED_ON_2: [u64; 3] = [3, 4, 5];
const LOCKED_ON_4: [u64; 4] = [6, 7, 8, 9];

#[tokio::test]
async fn an_assembled_certificate_passes() {
    let setup = Setup::new().await;
    assert!(setup.check(&setup.certificate()).is_ok());
}

#[tokio::test]
async fn groups_out_of_order_are_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    cert.earlier.swap(0, 1);
    setup.assert_refused(&cert, "not in strict lock order");
}

#[tokio::test]
async fn an_earlier_group_with_the_certificates_lock_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    cert.earlier[1].lock = cert.lock();
    setup.assert_refused(&cert, "not in strict lock order");
}

#[tokio::test]
async fn an_earlier_group_later_than_the_certificates_lock_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    cert.earlier[1].lock = Some(LockView {
        epoch: setup.epoch,
        view: ViewNumber::new(5),
    });
    setup.assert_refused(&cert, "not in strict lock order");
}

/// An earlier group cannot pass for the latest by dropping the lock
/// certificate: no lock sorts before every other.
#[tokio::test]
async fn a_certificate_without_its_lock_certificate_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    cert.latest.lock_cert = None;
    setup.assert_refused(&cert, "not in strict lock order");
}

/// Nor by naming another lock certificate: the latest group's signers signed
/// the lock of their own.
#[tokio::test]
async fn a_lock_certificate_the_latest_group_did_not_sign_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    cert.latest.lock_cert = Some(setup.cert1(3));
    setup.assert_refused(&cert, "signature check failed");
}

#[tokio::test]
async fn a_signer_in_two_groups_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    let twice = UNLOCKED[0] as usize;
    let (_, signers) = &mut cert.earlier[1].signatures;
    signers.set(twice, true);
    setup.assert_refused(&cert, "in two groups");
}

#[tokio::test]
async fn a_group_with_no_signer_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    let (_, signers) = &mut cert.earlier[0].signatures;
    signers.fill(false);
    setup.assert_refused(&cert, "has no signer");
}

#[tokio::test]
async fn a_signer_vector_of_another_length_is_refused() {
    let setup = Setup::new().await;
    let mut cert = setup.certificate();
    let (_, signers) = &mut cert.earlier[0].signatures;
    signers.push(false);
    setup.assert_refused(&cert, "does not match the stake table");
}

#[tokio::test]
async fn a_certificate_below_the_threshold_is_refused() {
    let setup = Setup::new().await;
    let cert = setup.certificate();
    let total: U256 = setup.stake_table.iter().map(|entry| entry.stake()).sum();
    let error = cert
        .check_signatures(
            &setup.stake_table,
            total + U256::ONE,
            &test_timeout_epoch_lock(),
        )
        .expect_err("short of the threshold");
    assert!(error.to_string().contains("below threshold"), "{error}");
}

struct Setup {
    epoch: EpochNumber,
    membership: EpochMembership<TestTypes>,
    stake_table: Vec<<BLSPubKey as SignatureKey>::StakeTableEntry>,
}

impl Setup {
    async fn new() -> Self {
        let epoch = EpochNumber::genesis();
        let membership = mock_membership()
            .membership_for_epoch(Some(epoch))
            .expect("the genesis epoch resolves");
        let stake_table = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
        Self {
            epoch,
            membership,
            stake_table,
        }
    }

    /// The three-group certificate the tests change.
    fn certificate(&self) -> TimeoutCertificate3<TestTypes> {
        let lock_2 = self.cert1(2);
        let lock_4 = self.cert1(4);
        let votes = self
            .votes(None, &UNLOCKED)
            .chain(self.votes(Some(LockView::of(&lock_2)), &LOCKED_ON_2))
            .chain(self.votes(Some(LockView::of(&lock_4)), &LOCKED_ON_4))
            .collect::<Vec<_>>();
        let cert = TimeoutCertificate3::assemble(
            ViewNumber::new(VIEW),
            self.epoch,
            &self.stake_table,
            votes,
            Some(lock_4),
        )
        .expect("assemble");
        assert_eq!(cert.earlier.len(), 2, "setup: two earlier groups");
        cert
    }

    /// A `Certificate1` at `view` over a block of its own.
    fn cert1(&self, view: u64) -> Certificate1<TestTypes> {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
        let leaf = RawCommitmentBuilder::new("leaf").u64(view).finalize();
        build_cert1(
            leaf,
            self.epoch,
            view,
            &self.membership,
            ViewNumber::new(view),
            &public_key,
            &private_key,
        )
    }

    /// The timeout votes of `signers`, each signing `lock`.
    fn votes(
        &self,
        lock: Option<LockView>,
        signers: &[u64],
    ) -> impl Iterator<
        Item = (
            Option<LockView>,
            BLSPubKey,
            <BLSPubKey as SignatureKey>::PureAssembledSignatureType,
        ),
    > {
        let data = TimeoutData3 {
            view: ViewNumber::new(VIEW),
            epoch: self.epoch,
            lock,
        };
        let upgrade_lock = test_timeout_epoch_lock::<TestTypes>();
        signers
            .iter()
            .map(move |index| {
                let (key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], *index);
                let vote = SimpleVote::create_signed_vote(
                    data.clone(),
                    ViewNumber::new(VIEW),
                    &key,
                    &private_key,
                    &upgrade_lock,
                )
                .expect("sign timeout vote");
                (lock, key, vote.signature())
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn check(&self, cert: &TimeoutCertificate3<TestTypes>) -> hotshot_utils::anytrace::Result<()> {
        cert.check_signatures(
            &self.stake_table,
            self.membership.success_threshold(),
            &test_timeout_epoch_lock(),
        )
    }

    /// `cert` fails the check whose message contains `reason`.
    fn assert_refused(&self, cert: &TimeoutCertificate3<TestTypes>, reason: &str) {
        let error = self.check(cert).expect_err("refused");
        assert!(
            error.to_string().contains(reason),
            "refused for another reason: {error}"
        );
    }
}
