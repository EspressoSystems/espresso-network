// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]
use std::{collections::BTreeMap, marker::PhantomData};

use bitvec::bitvec;
use committable::Committable;
use hotshot::{traits::BlockPayload, types::SignatureKey};
use hotshot_example_types::{block_types::TestTransaction, node_types::TestTypes};
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment, VidDisperse, VidDisperseAndDuration, VidDisperseShare,
        ViewNumber, vid_commitment,
    },
    epoch_membership::EpochMembership,
    message::{Proposal, UpgradeLock},
    simple_certificate::DaCertificate2,
    simple_vote::{DaData2, DaVote2, SimpleVote, VersionedVoteData, Voteable},
    stake_table::StakeTableEntries,
    traits::{EncodeBytes, node_implementation::NodeType},
    vote::{Certificate, HasViewNumber, Vote},
};
use vbs::version::Version;

pub type TestNodeKeyMap = BTreeMap<
    <TestTypes as NodeType>::SignatureKey,
    <<TestTypes as NodeType>::SignatureKey as SignatureKey>::PrivateKey,
>;

/// create certificate
/// # Panics
/// if we fail to sign the data
pub fn build_cert<
    TYPES: NodeType,
    DATAType: Voteable<TYPES> + 'static,
    VOTE: Vote<TYPES, Commitment = DATAType>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
>(
    data: DATAType,
    epoch_membership: &EpochMembership<TYPES>,
    view: ViewNumber,
    public_key: &TYPES::SignatureKey,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES>,
) -> CERT {
    let real_qc_sig = build_assembled_sig::<TYPES, VOTE, CERT, DATAType>(
        &data,
        epoch_membership,
        view,
        upgrade_lock,
    );

    let vote = SimpleVote::<TYPES, DATAType>::create_signed_vote(
        data,
        view,
        public_key,
        private_key,
        upgrade_lock,
    )
    .expect("Failed to sign data!");

    let vote_commitment =
        VersionedVoteData::new(vote.date().clone(), vote.view_number(), upgrade_lock)
            .expect("Failed to create VersionedVoteData!")
            .commit();

    CERT::create_signed_certificate(
        vote_commitment,
        vote.date().clone(),
        real_qc_sig,
        vote.view_number(),
    )
}

/// create signature
/// # Panics
/// if fails to convert node id into keypair
pub fn build_assembled_sig<
    TYPES: NodeType,
    VOTE: Vote<TYPES>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
    DATAType: Voteable<TYPES> + 'static,
>(
    data: &DATAType,
    epoch_membership: &EpochMembership<TYPES>,
    view: ViewNumber,
    upgrade_lock: &UpgradeLock<TYPES>,
) -> <TYPES::SignatureKey as SignatureKey>::QcType {
    let stake_table = CERT::stake_table(epoch_membership);
    let stake_table_entries = StakeTableEntries::<TYPES>::from(stake_table.clone()).0;
    let real_qc_pp: <TYPES::SignatureKey as SignatureKey>::QcParams<'_> =
        <TYPES::SignatureKey as SignatureKey>::public_parameter(
            &stake_table_entries,
            CERT::threshold(epoch_membership),
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
        .expect("Failed to sign data!");
        let original_signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType =
            vote.signature();
        sig_lists.push(original_signature);
    }

    <TYPES::SignatureKey as SignatureKey>::assemble(
        &real_qc_pp,
        signers.as_bitslice(),
        &sig_lists[..],
    )
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

pub async fn da_payload_commitment<TYPES: NodeType>(
    membership: &EpochMembership<TYPES>,
    transactions: Vec<TestTransaction>,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    version: Version,
) -> VidCommitment {
    let encoded_transactions = TestTransaction::encode(&transactions);

    vid_commitment(
        &encoded_transactions,
        &metadata.encode(),
        membership.total_nodes(),
        version,
    )
}

pub async fn build_vid_proposal<TYPES: NodeType>(
    membership: &EpochMembership<TYPES>,
    view_number: ViewNumber,
    epoch_number: Option<EpochNumber>,
    payload: &TYPES::BlockPayload,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES>,
) -> (
    Proposal<TYPES, VidDisperse<TYPES>>,
    Vec<Proposal<TYPES, VidDisperseShare<TYPES>>>,
) {
    let VidDisperseAndDuration {
        disperse: vid_disperse,
        duration: _,
    } = VidDisperse::calculate_vid_disperse(
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
        vid_disperse
            .to_shares()
            .into_iter()
            .map(|share| {
                share
                    .to_proposal(private_key)
                    .expect("Failed to sign payload commitment")
            })
            .collect(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_da_certificate<TYPES: NodeType>(
    membership: &EpochMembership<TYPES>,
    view_number: ViewNumber,
    epoch_number: Option<EpochNumber>,
    transactions: Vec<TestTransaction>,
    metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    public_key: &TYPES::SignatureKey,
    private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES>,
) -> anyhow::Result<DaCertificate2<TYPES>> {
    let encoded_transactions = TestTransaction::encode(&transactions);

    let da_payload_commitment = vid_commitment(
        &encoded_transactions,
        &metadata.encode(),
        membership.total_nodes(),
        upgrade_lock.version_infallible(view_number),
    );

    let next_epoch_da_payload_commitment =
        if upgrade_lock.epochs_enabled(view_number) && membership.epoch().is_some() {
            Some(vid_commitment(
                &encoded_transactions,
                &metadata.encode(),
                membership.next_epoch_stake_table()?.total_nodes(),
                upgrade_lock.version_infallible(view_number),
            ))
        } else {
            None
        };

    let da_data = DaData2 {
        payload_commit: da_payload_commitment,
        next_epoch_payload_commit: next_epoch_da_payload_commitment,
        epoch: epoch_number,
    };

    anyhow::Ok(build_cert::<
        TYPES,
        DaData2,
        DaVote2<TYPES>,
        DaCertificate2<TYPES>,
    >(
        da_data,
        membership,
        view_number,
        public_key,
        private_key,
        upgrade_lock,
    ))
}
