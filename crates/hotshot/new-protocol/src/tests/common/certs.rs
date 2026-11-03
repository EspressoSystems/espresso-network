use std::collections::BTreeMap;

use bitvec::bitvec;
use committable::Committable;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_vote::{SimpleVote, VersionedVoteData, Voteable},
    stake_table::StakeTableEntries,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vote::{Certificate, HasViewNumber, Vote},
};

pub type TestNodeKeyMap = BTreeMap<
    <TestTypes as NodeType>::SignatureKey,
    <<TestTypes as NodeType>::SignatureKey as SignatureKey>::PrivateKey,
>;

/// A certificate on `data` for `view`, signed by every member of `epoch_membership`'s committee,
/// whose own vote is cast with `public_key` and `private_key`.
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

/// The committee's assembled signature on `data`, with every node in the stake table signing in
/// stake-table order under the seed-derived key pair for its index.
fn build_assembled_sig<
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

fn key_pair_for_id<TYPES: NodeType>(
    node_id: u64,
) -> (
    <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    TYPES::SignatureKey,
) {
    let private_key = TYPES::SignatureKey::generated_from_seed_indexed([0u8; 32], node_id).1;
    let public_key = <TYPES as NodeType>::SignatureKey::from_private(&private_key);
    (private_key, public_key)
}
