// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]
use std::{collections::BTreeMap, fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

use alloy::primitives::U256;
use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use bitvec::bitvec;
use committable::Committable;
use hotshot::{
    traits::{BlockPayload, NodeImplementation, TestableNodeImplementation},
    types::{SignatureKey, SystemContextHandle},
    HotShotInitializer, SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider,
    block_types::TestTransaction,
    node_types::TestTypes,
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_task_impls::events::HotShotEvent;
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    data::{vid_commitment, Leaf2, VidCommitment, VidDisperse, VidDisperseShare},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::{Proposal, UpgradeLock},
    simple_certificate::DaCertificate2,
    simple_vote::{DaData2, DaVote2, SimpleVote, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::{
        election::Membership,
        node_implementation::{NodeType, Versions},
        storage::storage_add_drb_result,
        EncodeBytes,
    },
    utils::{option_epoch_from_block_number, View, ViewInner},
    vote::{Certificate, HasViewNumber, Vote},
    ValidatorConfig,
};
use serde::Serialize;
use vbs::version::Version;

use crate::{test_builder::TestDescription, test_launcher::TestLauncher};

pub type TestNodeKeyMap = BTreeMap<
    <TestTypes as NodeType>::SignatureKey,
    <<TestTypes as NodeType>::SignatureKey as SignatureKey>::PrivateKey,
>;

/// create the [`SystemContextHandle`] from a node id, with no epochs
/// # Panics
/// if cannot create a [`HotShotInitializer`]
pub async fn build_system_handle<
    TYPES: NodeType<InstanceState = TestInstanceState>,
    I: NodeImplementation<
            TYPES,
            Storage = TestStorage<TYPES>,
            AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
        > + TestableNodeImplementation<TYPES>,
    V: Versions,
>(
    node_id: u64,
) -> (
    SystemContextHandle<TYPES, I, V>,
    Sender<Arc<HotShotEvent<TYPES>>>,
    Receiver<Arc<HotShotEvent<TYPES>>>,
    Arc<TestNodeKeyMap>,
) {
    let builder: TestDescription<TYPES, I, V> = TestDescription::default_multiple_rounds();

    let launcher = builder.gen_launcher().map_hotshot_config(|hotshot_config| {
        hotshot_config.epoch_height = 0;
    });
    build_system_handle_from_launcher(node_id, &launcher).await
}

/// create the [`SystemContextHandle`] from a node id and `TestLauncher`
/// # Panics
/// if cannot create a [`HotShotInitializer`]
pub async fn build_system_handle_from_launcher<
    TYPES: NodeType<InstanceState = TestInstanceState>,
    I: NodeImplementation<
            TYPES,
            Storage = TestStorage<TYPES>,
            AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
        > + TestableNodeImplementation<TYPES>,
    V: Versions,
>(
    node_id: u64,
    launcher: &TestLauncher<TYPES, I, V>,
) -> (
    SystemContextHandle<TYPES, I, V>,
    Sender<Arc<HotShotEvent<TYPES>>>,
    Receiver<Arc<HotShotEvent<TYPES>>>,
    Arc<TestNodeKeyMap>,
) {
    let network = (launcher.resource_generators.channel_generator)(node_id).await;
    let storage = (launcher.resource_generators.storage)(node_id);
    let marketplace_config = (launcher.resource_generators.marketplace_config)(node_id);
    let hotshot_config = (launcher.resource_generators.hotshot_config)(node_id);

    let initializer = HotShotInitializer::<TYPES>::from_genesis::<V>(
        TestInstanceState::new(launcher.metadata.async_delay_config.clone()),
        launcher.metadata.test_config.epoch_height,
        launcher.metadata.test_config.epoch_start_block,
        vec![],
    )
    .await
    .unwrap();

    // See whether or not we should be DA
    let is_da = node_id < hotshot_config.da_staked_committee_size as u64;

    // We assign node's public key and stake value rather than read from config file since it's a test
    let validator_config: ValidatorConfig<TYPES> =
        ValidatorConfig::generated_from_seed_indexed([0u8; 32], node_id, U256::from(1), is_da);
    let private_key = validator_config.private_key.clone();
    let public_key = validator_config.public_key.clone();
    let state_private_key = validator_config.state_private_key.clone();

    let memberships = Arc::new(RwLock::new(TYPES::Membership::new(
        hotshot_config.known_nodes_with_stake.clone(),
        hotshot_config.known_da_nodes.clone(),
    )));

    let coordinator = EpochMembershipCoordinator::new(
        memberships,
        Some(storage_add_drb_result(storage.clone())),
        hotshot_config.epoch_height,
    );
    let node_key_map = launcher.metadata.build_node_key_map();

    let (c, s, r) = SystemContext::init(
        public_key,
        private_key,
        state_private_key,
        node_id,
        hotshot_config,
        coordinator,
        network,
        initializer,
        ConsensusMetricsValue::default(),
        storage,
        marketplace_config,
    )
    .await
    .expect("Could not init hotshot");

    (c, s, r, node_key_map)
}

/// create certificate
/// # Panics
/// if we fail to sign the data
pub async fn build_cert<
    TYPES: NodeType,
    V: Versions,
    DATAType: Committable + Clone + Eq + Hash + Serialize + Debug + 'static,
    VOTE: Vote<TYPES, Commitment = DATAType>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
>(
    data: DATAType,
    epoch_membership: &EpochMembership<TYPES>,
    view: TYPES::View,
    public_key: &TYPES::SignatureKey,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES, V>,
) -> CERT {
    let real_qc_sig = build_assembled_sig::<TYPES, V, VOTE, CERT, DATAType>(
        &data,
        epoch_membership,
        view,
        upgrade_lock,
    )
    .await;

    let vote = SimpleVote::<TYPES, DATAType>::create_signed_vote(
        data,
        view,
        public_key,
        private_key,
        upgrade_lock,
    )
    .await
    .expect("Failed to sign data!");

    let vote_commitment =
        VersionedVoteData::new(vote.date().clone(), vote.view_number(), upgrade_lock)
            .await
            .expect("Failed to create VersionedVoteData!")
            .commit();

    let cert = CERT::create_signed_certificate(
        vote_commitment,
        vote.date().clone(),
        real_qc_sig,
        vote.view_number(),
    );
    cert
}

pub fn vid_share<TYPES: NodeType>(
    shares: &[Proposal<TYPES, VidDisperseShare<TYPES>>],
    pub_key: TYPES::SignatureKey,
) -> Proposal<TYPES, VidDisperseShare<TYPES>> {
    shares
        .iter()
        .filter(|s| *s.data.recipient_key() == pub_key)
        .cloned()
        .collect::<Vec<_>>()
        .first()
        .expect("No VID for key")
        .clone()
}

/// create signature
/// # Panics
/// if fails to convert node id into keypair
pub async fn build_assembled_sig<
    TYPES: NodeType,
    V: Versions,
    VOTE: Vote<TYPES>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
    DATAType: Committable + Clone + Eq + Hash + Serialize + Debug + 'static,
>(
    data: &DATAType,
    epoch_membership: &EpochMembership<TYPES>,
    view: TYPES::View,
    upgrade_lock: &UpgradeLock<TYPES, V>,
) -> <TYPES::SignatureKey as SignatureKey>::QcType {
    let stake_table = CERT::stake_table(epoch_membership).await;
    let stake_table_entries = StakeTableEntries::<TYPES>::from(stake_table.clone()).0;
    let real_qc_pp: <TYPES::SignatureKey as SignatureKey>::QcParams<'_> =
        <TYPES::SignatureKey as SignatureKey>::public_parameter(
            &stake_table_entries,
            CERT::threshold(epoch_membership).await,
        );

    let total_nodes = stake_table.len();
    let signers = bitvec![1; total_nodes];
    let mut sig_lists = Vec::new();

    // assemble the vote
    for node_id in 0..total_nodes {
        let (private_key_i, public_key_i) = key_pair_for_id::<TYPES>(node_id.try_into().unwrap());
        let vote: SimpleVote<TYPES, DATAType> = SimpleVote::<TYPES, DATAType>::create_signed_vote(
            data.clone(),
            view,
            &public_key_i,
            &private_key_i,
            upgrade_lock,
        )
        .await
        .expect("Failed to sign data!");
        let original_signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType =
            vote.signature();
        sig_lists.push(original_signature);
    }

    let real_qc_sig = <TYPES::SignatureKey as SignatureKey>::assemble(
        &real_qc_pp,
        signers.as_bitslice(),
        &sig_lists[..],
    );

    real_qc_sig
}

/// get the keypair for a node id
#[must_use]
pub fn key_pair_for_id<TYPES: NodeType>(
    node_id: u64,
) -> (
    <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    TYPES::SignatureKey,
) {
    let private_key = TYPES::SignatureKey::generated_from_seed_indexed([0u8; 32], node_id).1;
    let public_key = <TYPES as NodeType>::SignatureKey::from_private(&private_key);
    (private_key, public_key)
}

pub async fn da_payload_commitment<TYPES: NodeType, V: Versions>(
    membership: &EpochMembership<TYPES>,
    transactions: Vec<TestTransaction>,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    version: Version,
) -> VidCommitment {
    let encoded_transactions = TestTransaction::encode(&transactions);

    vid_commitment::<V>(
        &encoded_transactions,
        &metadata.encode(),
        membership.total_nodes().await,
        version,
    )
}

pub async fn build_payload_commitment<TYPES: NodeType, V: Versions>(
    membership: &EpochMembership<TYPES>,
    view: TYPES::View,
    version: Version,
) -> VidCommitment {
    // Make some empty encoded transactions, we just care about having a commitment handy for the
    // later calls. We need the VID commitment to be able to propose later.
    let encoded_transactions = Vec::new();
    let num_storage_nodes = membership.committee_members(view).await.len();
    vid_commitment::<V>(&encoded_transactions, &[], num_storage_nodes, version)
}

pub async fn build_vid_proposal<TYPES: NodeType, V: Versions>(
    membership: &EpochMembership<TYPES>,
    view_number: TYPES::View,
    epoch_number: Option<TYPES::Epoch>,
    payload: &TYPES::BlockPayload,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES, V>,
) -> (
    Proposal<TYPES, VidDisperse<TYPES>>,
    Vec<Proposal<TYPES, VidDisperseShare<TYPES>>>,
) {
    let vid_disperse = VidDisperse::calculate_vid_disperse::<V>(
        payload,
        &membership.coordinator,
        view_number,
        epoch_number,
        epoch_number,
        metadata,
        upgrade_lock,
    )
    .await
    .unwrap();

    let signature =
        TYPES::SignatureKey::sign(private_key, vid_disperse.payload_commitment().as_ref())
            .expect("Failed to sign VID commitment");
    let vid_disperse_proposal = Proposal {
        data: vid_disperse.clone(),
        signature,
        _pd: PhantomData,
    };

    (
        vid_disperse_proposal,
        VidDisperseShare::from_vid_disperse(vid_disperse)
            .into_iter()
            .map(|vid_disperse| {
                vid_disperse
                    .to_proposal(private_key)
                    .expect("Failed to sign payload commitment")
            })
            .collect(),
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn build_da_certificate<TYPES: NodeType, V: Versions>(
    membership: &EpochMembership<TYPES>,
    view_number: TYPES::View,
    epoch_number: Option<TYPES::Epoch>,
    transactions: Vec<TestTransaction>,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    public_key: &TYPES::SignatureKey,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES, V>,
) -> anyhow::Result<DaCertificate2<TYPES>> {
    let encoded_transactions = TestTransaction::encode(&transactions);

    let da_payload_commitment = vid_commitment::<V>(
        &encoded_transactions,
        &metadata.encode(),
        membership.total_nodes().await,
        upgrade_lock.version_infallible(view_number).await,
    );

    let next_epoch_da_payload_commitment =
        if upgrade_lock.epochs_enabled(view_number).await && membership.epoch().is_some() {
            Some(vid_commitment::<V>(
                &encoded_transactions,
                &metadata.encode(),
                membership
                    .next_epoch_stake_table()
                    .await?
                    .total_nodes()
                    .await,
                upgrade_lock.version_infallible(view_number).await,
            ))
        } else {
            None
        };

    let da_data = DaData2 {
        payload_commit: da_payload_commitment,
        next_epoch_payload_commit: next_epoch_da_payload_commitment,
        epoch: epoch_number,
    };

    anyhow::Ok(
        build_cert::<TYPES, V, DaData2<TYPES>, DaVote2<TYPES>, DaCertificate2<TYPES>>(
            da_data,
            membership,
            view_number,
            public_key,
            private_key,
            upgrade_lock,
        )
        .await,
    )
}

/// This function permutes the provided input vector `inputs`, given some order provided within the
/// `order` vector.
///
/// # Examples
/// let output = permute_input_with_index_order(vec![1, 2, 3], vec![2, 1, 0]);
/// // Output is [3, 2, 1] now
pub fn permute_input_with_index_order<T>(inputs: Vec<T>, order: Vec<usize>) -> Vec<T>
where
    T: Clone,
{
    let mut ordered_inputs = Vec::with_capacity(inputs.len());
    for &index in &order {
        ordered_inputs.push(inputs[index].clone());
    }
    ordered_inputs
}

/// This function will create a fake [`View`] from a provided [`Leaf`].
pub async fn build_fake_view_with_leaf<V: Versions>(
    leaf: Leaf2<TestTypes>,
    upgrade_lock: &UpgradeLock<TestTypes, V>,
    epoch_height: u64,
) -> View<TestTypes> {
    build_fake_view_with_leaf_and_state(
        leaf,
        TestValidatedState::default(),
        upgrade_lock,
        epoch_height,
    )
    .await
}

/// This function will create a fake [`View`] from a provided [`Leaf`] and `state`.
pub async fn build_fake_view_with_leaf_and_state<V: Versions>(
    leaf: Leaf2<TestTypes>,
    state: TestValidatedState,
    _upgrade_lock: &UpgradeLock<TestTypes, V>,
    epoch_height: u64,
) -> View<TestTypes> {
    let epoch =
        option_epoch_from_block_number::<TestTypes>(leaf.with_epoch, leaf.height(), epoch_height);
    View {
        view_inner: ViewInner::Leaf {
            leaf: leaf.commit(),
            state: state.into(),
            delta: None,
            epoch,
        },
    }
}
