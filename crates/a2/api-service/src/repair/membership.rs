//! The stake tables of the chain, which repair derives as consensus does: the genesis file
//! gives the stake table of the first epochs, and each later one comes from the stake table
//! contract's events on L1 up to the root block of the epoch two before. The root blocks come from
//! the store, so the stake tables follow what the store holds.
//!
//! Repair needs them to check what the chain does not commit to, the `Certificate2` that
//! decided a block directly and the state certificate of an epoch root, to apply the rewards an
//! epoch pays, and to check discovery records. It keeps them, and the L1 events they come from, in
//! the store, and publishes the stake tables there for the API instances.

use std::{ops::Range, sync::Arc};

use anyhow::{Context, bail, ensure};
use chain_store::ChainStore;
use committable::Committable;
use espresso_genesis::Genesis;
use espresso_types::{Header, Leaf2, NodeState, SeqTypes};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
    stake_table::HSStakeTable,
    traits::{
        election::{Membership as _, MembershipSnapshot as _},
        metrics::NoMetrics,
    },
    utils::{epoch_from_block_number, is_epoch_root, root_block_in_epoch},
};
use light_client::consensus::quorum::{Quorum, StakeTable, StakeTableQuorum};
use membership::{MembershipStore, null_catchup::NullStateCatchup};
use url::Url;
use versions::Upgrade;

use crate::repair::state_cert::validate_state_cert;

/// The node state of the chain `genesis` starts, as repair keeps it: stake tables in
/// `store`, read from the L1 at `l1`, and no catchup, since repair follows the chain
/// itself. A restart finds the stake tables and L1 events in the store, and reads from L1 only what
/// is newer.
pub async fn node_state(
    genesis: &Genesis,
    l1: Vec<Url>,
    store: ChainStore,
) -> anyhow::Result<NodeState> {
    let network_config = genesis.unstaked_network_config()?;
    membership::node_state::node_state(
        genesis,
        &network_config,
        l1,
        MembershipStore::new(store),
        Arc::new(NullStateCatchup::default()),
        &NoMetrics,
    )
    .await
}

/// The stake tables of the chain, as far as the store's epoch roots reach.
#[derive(Clone)]
pub struct Membership {
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
}

impl Membership {
    /// The membership `state` keeps.
    pub fn new(state: &NodeState) -> anyhow::Result<Self> {
        Ok(Self {
            coordinator: state.coordinator.clone(),
            epoch_height: state
                .epoch_height
                .filter(|&height| height > 0)
                .context("the chain needs a positive epoch_height")?,
        })
    }

    pub fn epoch_height(&self) -> u64 {
        self.epoch_height
    }

    /// The first epoch, which runs on the genesis stake table.
    pub fn first_epoch(&self) -> EpochNumber {
        self.coordinator.membership().first_epoch()
    }

    /// The epoch of the block at `height`.
    pub fn epoch_of(&self, height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(height, self.epoch_height))
    }

    /// The heights of the blocks of `epoch`.
    pub fn heights_of(&self, epoch: EpochNumber) -> Range<u64> {
        epoch_heights(epoch, self.epoch_height)
    }

    /// The height of the root block inside `epoch`, which gives the stake table of the epoch two
    /// later.
    pub fn root_of(&self, epoch: EpochNumber) -> u64 {
        root_block_in_epoch(*epoch, self.epoch_height)
    }

    /// Derives the stake table of the epoch two after `root`'s, from L1. `preceding` is the root of
    /// the epoch before `root`'s, if the store has it; without it the block reward assumes the
    /// block time, which the checks here do not depend on.
    pub async fn install_root(
        &self,
        root: Header,
        preceding: Option<Header>,
    ) -> anyhow::Result<()> {
        ensure!(
            is_epoch_root(root.height(), self.epoch_height),
            "block {} is no epoch root",
            root.height()
        );
        self.coordinator
            .membership()
            .install_epoch_roots(preceding, vec![root], &self.coordinator)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    /// The stake table of `epoch`, if it is known.
    pub fn stake_table(&self, epoch: EpochNumber) -> Option<HSStakeTable<SeqTypes>> {
        let snapshot = self.coordinator.membership().snapshot(epoch)?;
        Some(HSStakeTable::from_iter(snapshot.stake_table()))
    }

    /// Checks that `cert2` decides `leaf`, and has a quorum of the stake table of `leaf`'s epoch.
    pub async fn check_cert2(
        &self,
        cert2: &Certificate2<SeqTypes>,
        leaf: &Leaf2,
    ) -> anyhow::Result<()> {
        let height = leaf.height();
        ensure!(
            cert2.data.block_number == height && cert2.data.leaf_commit == leaf.commit(),
            "a cert2 of block {} for block {height}",
            cert2.data.block_number
        );
        let epoch = self.epoch_of(height);
        ensure!(
            cert2.data.epoch == epoch,
            "a cert2 of epoch {} for a block of epoch {epoch}",
            cert2.data.epoch
        );
        let Some(table) = self.stake_table(epoch) else {
            bail!("no stake table of epoch {epoch}");
        };
        let table = Arc::new(StakeTable::from(table));
        StakeTableQuorum::new((table.clone(), table), self.epoch_height)
            .verify_cert2(cert2, leaf.block_header().version())
            .await
    }

    /// Checks that `cert`, the state certificate of the epoch root with `header`, has the
    /// signatures of the stake table of its epoch.
    pub fn check_state_cert(
        &self,
        cert: &LightClientStateUpdateCertificateV2<SeqTypes>,
        header: &Header,
    ) -> anyhow::Result<()> {
        let epoch = self.epoch_of(header.height());
        let Some(table) = self.stake_table(epoch) else {
            bail!("no stake table of epoch {epoch}");
        };
        // The block's version is the one its certificate was signed under.
        let lock = UpgradeLock::new(Upgrade::trivial(header.version()));
        validate_state_cert(cert, &table, epoch, self.epoch_height, &lock)
    }
}

/// The heights of the blocks of `epoch`, in epochs of `epoch_height` blocks: epoch 1 has blocks 1
/// up to `epoch_height`.
fn epoch_heights(epoch: EpochNumber, epoch_height: u64) -> Range<u64> {
    let end = *epoch * epoch_height + 1;
    end.saturating_sub(epoch_height)..end
}

#[cfg(test)]
pub(crate) mod testing {
    //! A membership on a fixed committee, which needs no L1, and certificates it checks.

    use std::num::NonZeroU64;

    use alloy::primitives::{FixedBytes, U256};
    use async_lock::Mutex as AsyncMutex;
    use bitvec::vec::BitVec;
    use chain_store::mem::MemBackend;
    use espresso_types::{
        EpochCommittees, L1ClientOptions, NodeState, PubKey, ValidatedState,
        v0_3::{ChainConfig, Fetcher},
    };
    use hotshot_contract_adapter::light_client::derive_signed_state_digest;
    use hotshot_types::{
        PeerConfig,
        light_client::{
            CircuitField, LightClientState, StakeTableState, StateKeyPair, StateVerKey,
        },
        simple_certificate::SimpleCertificate,
        simple_vote::{VersionedVoteData, Vote2Data},
        traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey},
        vote::Certificate,
    };
    use membership::null_catchup::NullStateCatchup;
    use versions::NEW_PROTOCOL_VERSION;

    use super::*;

    pub(crate) const EPOCH_HEIGHT: u64 = 10;
    pub(crate) const SIGNERS: u64 = 4;
    pub(crate) const STAKE: u64 = 100;

    /// The first two epochs run on the genesis committee, so this membership needs no L1.
    pub(crate) const FIRST_EPOCH: u64 = 1;

    pub(crate) fn bls_key(i: u64) -> (PubKey, <PubKey as SignatureKey>::PrivateKey) {
        PubKey::generated_from_seed_indexed([0; 32], i)
    }

    pub(crate) fn state_key(i: u64) -> StateKeyPair {
        StateKeyPair::generate_from_seed_indexed([0; 32], i)
    }

    pub(crate) fn committee() -> Vec<PeerConfig<SeqTypes>> {
        (0..SIGNERS)
            .map(|i| PeerConfig {
                stake_table_entry: bls_key(i).0.stake_table_entry(U256::from(STAKE)),
                state_ver_key: state_key(i).ver_key(),
                connect_info: None,
            })
            .collect()
    }

    pub(crate) fn membership() -> Membership {
        // Never reached: the genesis committee's epochs read nothing from L1.
        let l1 = L1ClientOptions::default()
            .connect(vec!["http://127.0.0.1:1".parse().unwrap()])
            .unwrap();
        let store = MembershipStore::new(ChainStore::new(MemBackend::default()));
        let fetcher = Fetcher::new(
            Arc::new(NullStateCatchup::default()),
            Arc::new(AsyncMutex::new(store.clone())),
            l1,
            ChainConfig::default(),
        );
        let committees = EpochCommittees::new(
            committee(),
            committee(),
            None,
            fetcher,
            EPOCH_HEIGHT,
            EpochNumber::new(FIRST_EPOCH),
        )
        .unwrap();
        Membership {
            coordinator: EpochMembershipCoordinator::new(
                committees,
                NonZeroU64::new(EPOCH_HEIGHT).unwrap(),
                &store,
            ),
            epoch_height: EPOCH_HEIGHT,
        }
    }

    /// A leaf of the new protocol at `height`.
    pub(crate) async fn leaf(height: u64) -> Leaf2 {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let mut leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        *leaf.block_header_mut().height_mut() = height;
        leaf
    }

    /// A cert2 deciding `leaf`, labelled `epoch`, signed by the keys `signers`.
    pub(crate) fn cert2(leaf: &Leaf2, epoch: u64, signers: &[u64]) -> Certificate2<SeqTypes> {
        let data = Vote2Data {
            leaf_commit: leaf.commit(),
            epoch: EpochNumber::new(epoch),
            block_number: leaf.height(),
        };
        let view = hotshot_types::data::ViewNumber::new(7);
        let lock = UpgradeLock::<SeqTypes>::new(Upgrade::trivial(leaf.block_header().version()));
        let commitment = VersionedVoteData::new_infallible(data.clone(), view, &lock).commit();
        let entries: Vec<_> = committee()
            .iter()
            .map(|peer| peer.stake_table_entry.clone())
            .collect();
        // Assembling checks only that the signers reach this threshold; the check under test
        // applies the stake table's own.
        let pp = PubKey::public_parameter(&entries, U256::ONE);
        let signed: BitVec = (0..SIGNERS).map(|i| signers.contains(&i)).collect();
        let signatures: Vec<_> = signers
            .iter()
            .map(|&i| PubKey::sign(&bls_key(i).1, commitment.as_ref()).unwrap())
            .collect();
        let signature = PubKey::assemble(&pp, &signed, &signatures);
        SimpleCertificate::create_signed_certificate(commitment, data, signature, view)
    }

    /// A state certificate for the block at `height`, signed by the state keys `signers`.
    pub(crate) fn state_cert(
        height: u64,
        signers: &[u64],
    ) -> LightClientStateUpdateCertificateV2<SeqTypes> {
        let light_client_state = LightClientState {
            view_number: 7,
            block_height: height,
            block_comm_root: CircuitField::from(7u64),
        };
        let next_stake_table_state = StakeTableState {
            bls_key_comm: CircuitField::from(1u64),
            schnorr_key_comm: CircuitField::from(2u64),
            amount_comm: CircuitField::from(3u64),
            threshold: CircuitField::from(1u64),
        };
        let auth_root = FixedBytes::<32>::from([9u8; 32]);
        let digest =
            derive_signed_state_digest(&light_client_state, &next_stake_table_state, &auth_root);
        let signatures = signers
            .iter()
            .map(|&i| {
                let keys = state_key(i);
                let lcv2 = <StateVerKey as LCV2StateSignatureKey>::sign_state(
                    keys.sign_key_ref(),
                    &light_client_state,
                    &next_stake_table_state,
                )
                .unwrap();
                let lcv3 =
                    <StateVerKey as LCV3StateSignatureKey>::sign_state(keys.sign_key_ref(), digest)
                        .unwrap();
                (keys.ver_key(), lcv3, lcv2)
            })
            .collect();
        LightClientStateUpdateCertificateV2 {
            epoch: EpochNumber::new(epoch_from_block_number(height, EPOCH_HEIGHT)),
            light_client_state,
            next_stake_table_state,
            signatures,
            auth_root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{testing::*, *};

    #[tokio::test]
    async fn checks_a_cert2_against_the_stake_table_of_its_epoch() {
        let membership = membership();
        let epoch = FIRST_EPOCH + 1;
        let leaf = leaf(membership.heights_of(EpochNumber::new(epoch)).start + 2).await;

        membership
            .check_cert2(&cert2(&leaf, epoch, &[0, 1, 2]), &leaf)
            .await
            .expect("a quorum of the stake table");
        membership
            .check_cert2(&cert2(&leaf, epoch, &[0, 1]), &leaf)
            .await
            .expect_err("half the stake is no quorum");
        membership
            .check_cert2(&cert2(&leaf, epoch - 1, &[0, 1, 2]), &leaf)
            .await
            .expect_err("a cert2 labelled with another epoch");
        let other = self::leaf(leaf.height() + 1).await;
        membership
            .check_cert2(&cert2(&other, epoch, &[0, 1, 2]), &leaf)
            .await
            .expect_err("a cert2 of another leaf");

        let later = self::leaf(membership.heights_of(EpochNumber::new(epoch + 3)).start).await;
        membership
            .check_cert2(&cert2(&later, epoch + 3, &[0, 1, 2]), &later)
            .await
            .expect_err("no stake table of the epoch yet");
    }

    #[tokio::test]
    async fn checks_a_state_cert_against_the_stake_table_of_its_epoch() {
        let membership = membership();
        let epoch = EpochNumber::new(FIRST_EPOCH + 1);
        let root = membership.root_of(epoch);
        let header = leaf(root).await.block_header().clone();

        membership
            .check_state_cert(&state_cert(root, &[0, 1]), &header)
            .expect("more than a third of the stake");
        membership
            .check_state_cert(&state_cert(root, &[0]), &header)
            .expect_err("a quarter of the stake is too little");
        let other = leaf(root - 1).await.block_header().clone();
        membership
            .check_state_cert(&state_cert(root - 1, &[0, 1]), &other)
            .expect_err("a block that is no epoch root");
    }

    #[test]
    fn epoch_heights_agree_with_the_epoch_of_each_height() {
        let epoch_height = 100;
        for epoch in [1, 2, 37] {
            let epoch = EpochNumber::new(epoch);
            let heights = epoch_heights(epoch, epoch_height);
            assert_eq!(heights.end - heights.start, epoch_height);
            for height in [heights.start, heights.end - 1] {
                assert_eq!(epoch_from_block_number(height, epoch_height), *epoch);
            }
            assert_eq!(
                epoch_from_block_number(heights.end, epoch_height),
                *epoch + 1
            );
            assert!(heights.contains(&root_block_in_epoch(*epoch, epoch_height)));
        }
    }
}
