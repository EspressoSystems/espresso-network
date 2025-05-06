// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#[cfg(test)]
use std::marker::PhantomData;

use committable::Committable;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    message::{GeneralConsensusMessage, Message, MessageKind, SequencingMessage},
    signature_key::BLSPubKey,
    simple_certificate::SimpleCertificate,
    simple_vote::ViewSyncCommitData2,
    traits::{node_implementation::ConsensusTime, signature_key::SignatureKey},
};
use vbs::{
    version::{StaticVersion, Version},
    BinarySerializer, Serializer,
};

#[test]
// Checks that the current program protocol version
// correctly appears at the start of a serialized messaged.
fn version_number_at_start_of_serialization() {
    let sender = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let view_number = ConsensusTime::new(17);
    let epoch = None;
    // The version we set for the message
    const MAJOR: u16 = 37;
    const MINOR: u16 = 17;
    let version = Version {
        major: MAJOR,
        minor: MINOR,
    };
    type TestVersion = StaticVersion<MAJOR, MINOR>;
    // The specific data we attach to our message shouldn't affect the serialization,
    // we're using ViewSyncCommitData for simplicity.
    let data: ViewSyncCommitData2<TestTypes> = ViewSyncCommitData2 {
        relay: 37,
        round: view_number,
        epoch,
    };
    let simple_certificate =
        SimpleCertificate::new(data.clone(), data.commit(), view_number, None, PhantomData);
    let message = Message {
        sender,
        kind: MessageKind::Consensus(SequencingMessage::General(
            GeneralConsensusMessage::ViewSyncCommitCertificate2(simple_certificate),
        )),
    };
    let serialized_message: Vec<u8> = Serializer::<TestVersion>::serialize(&message).unwrap();
    // The versions we've read from the message

    let version_read = Version::deserialize(&serialized_message).unwrap().0;

    assert_eq!(version.major, version_read.major);
    assert_eq!(version.minor, version_read.minor);
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_certificate2_validity() {
    use futures::StreamExt;
    use hotshot_example_types::node_types::{MemoryImpl, TestTypes, TestVersions};
    use hotshot_testing::{helpers::build_system_handle, view_generator::TestViewGenerator};
    use hotshot_types::{
        data::{Leaf, Leaf2},
        vote::Certificate,
    };

    hotshot::helpers::initialize_logging();

    let node_id = 1;

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;
    let membership = handle.hotshot.membership_coordinator.clone();

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();

    for view in (&mut generator).take(4).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());
    }

    let proposal = proposals[3].clone();
    let parent_proposal = proposals[2].clone();

    // ensure that we don't break certificate validation
    let qc2 = proposal.data.justify_qc().clone();
    let qc = qc2.clone().to_qc();

    let epoch_mem = membership.membership_for_epoch(None).await.unwrap();
    let membership_stake_table = epoch_mem
        .stake_table()
        .await
        .into_iter()
        .map(|config| config.stake_table_entry)
        .collect::<Vec<_>>();
    let membership_success_threshold = epoch_mem.success_threshold().await;

    assert!(qc
        .is_valid_cert(
            &membership_stake_table,
            membership_success_threshold,
            &handle.hotshot.upgrade_lock
        )
        .await
        .is_ok());

    assert!(qc2
        .is_valid_cert(
            &membership_stake_table,
            membership_success_threshold,
            &handle.hotshot.upgrade_lock
        )
        .await
        .is_ok());

    // ensure that we don't break the leaf commitment chain
    let leaf2 = Leaf2::from_quorum_proposal(&proposal.data);
    let parent_leaf2 = Leaf2::from_quorum_proposal(&parent_proposal.data);

    let leaf = Leaf::from_quorum_proposal(&proposal.data.into());
    let parent_leaf = Leaf::from_quorum_proposal(&parent_proposal.data.into());

    assert!(leaf.parent_commitment() == parent_leaf.commit(&handle.hotshot.upgrade_lock).await);

    assert!(leaf2.parent_commitment() == parent_leaf2.commit());
}
