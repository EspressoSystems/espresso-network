use std::{fmt::Debug, marker::PhantomData};

use committable::Committable;
use data_source::testing::TestableSequencerDataSource;
use espresso_types::{
    Header, Leaf2, MOCK_SEQUENCER_VERSIONS, NamespaceId, NamespaceProofQueryData, ValidatedState,
    traits::{EventConsumer, PersistenceOptions},
};
use futures::{future, stream::StreamExt};
use hotshot_example_types::node_types::TEST_VERSIONS;
use hotshot_query_service::availability::{
    AvailabilityDataSource, BlockQueryData, VidCommonQueryData,
};
use hotshot_types::{
    data::{
        DaProposal2, EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare, ns_table::parse_ns_table, vid_disperse::AvidMDisperseShare,
    },
    event::LeafInfo,
    message::Proposal,
    simple_certificate::{CertificatePair, QuorumCertificate2},
    traits::{EncodeBytes, signature_key::SignatureKey},
    utils::EpochTransitionIndicator,
    vid::avidm::{AvidMScheme, init_avidm_param},
};
use surf_disco::Client;
use test_helpers::{
    TestNetwork, TestNetworkConfigBuilder, catchup_test_helper, state_signature_test_helper,
    status_test_helper, submit_test_helper,
};
use test_utils::reserve_tcp_port;
use tide_disco::error::ServerError;
use vbs::version::StaticVersion;

use super::{update::ApiEventConsumer, *};
use crate::{
    network,
    persistence::no_storage::NoStorage,
    testing::{TestConfigBuilder, wait_for_decide_on_handle},
};

#[rstest_reuse::template]
#[rstest::rstest]
#[case(PhantomData::<crate::api::sql::DataSource>)]
#[case(PhantomData::<crate::api::fs::DataSource>)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub fn testable_sequencer_data_source<D: TestableSequencerDataSource>(#[case] _d: PhantomData<D>) {}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub(crate) async fn submit_test_with_query_module<D: TestableSequencerDataSource>(
    _d: PhantomData<D>,
) {
    let storage = D::create_storage().await;
    submit_test_helper(|opt| D::options(&storage, opt)).await
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub(crate) async fn status_test_with_query_module<D: TestableSequencerDataSource>(
    _d: PhantomData<D>,
) {
    let storage = D::create_storage().await;
    status_test_helper(|opt| D::options(&storage, opt)).await
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub(crate) async fn state_signature_test_with_query_module<D: TestableSequencerDataSource>(
    _d: PhantomData<D>,
) {
    let storage = D::create_storage().await;
    state_signature_test_helper(|opt| D::options(&storage, opt)).await
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub(crate) async fn test_namespace_query<D: TestableSequencerDataSource>(_d: PhantomData<D>) {
    // Arbitrary transaction, arbitrary namespace ID
    let ns_id = NamespaceId::from(42_u32);
    let txn = Transaction::new(ns_id, vec![1, 2, 3, 4]);

    // Start query service.
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let storage = D::create_storage().await;
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(D::options(&storage, Options::with_port(port)).submit(Default::default()))
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();

    // Connect client.
    let client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{port}").parse().unwrap());
    client.connect(None).await;

    let hash = client
        .post("submit/submit")
        .body_json(&txn)
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(txn.commit(), hash);

    // Wait for a Decide event containing transaction matching the one we sent
    let block_height = wait_for_decide_on_handle(&mut events, &txn).await.0 as usize;
    tracing::info!(block_height, "transaction sequenced");

    // Submit a second transaction for range queries.
    let txn2 = Transaction::new(ns_id, vec![5, 6, 7, 8]);
    client
        .post::<Commitment<Transaction>>("submit/submit")
        .body_json(&txn2)
        .unwrap()
        .send()
        .await
        .unwrap();
    let block_height2 = wait_for_decide_on_handle(&mut events, &txn2).await.0 as usize;
    tracing::info!(block_height2, "transaction sequenced");

    // Wait for the query service to update to this block height.
    client
        .socket(&format!("availability/stream/blocks/{block_height2}"))
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .next()
        .await
        .unwrap()
        .unwrap();

    let mut found_txn = false;
    let mut found_empty_block = false;
    for block_num in 0..=block_height {
        let header: Header = client
            .get(&format!("availability/header/{block_num}"))
            .send()
            .await
            .unwrap();
        let ns_query_res: NamespaceProofQueryData = client
            .get(&format!("availability/block/{block_num}/namespace/{ns_id}"))
            .send()
            .await
            .unwrap();

        // Check other means of querying the same proof.
        assert_eq!(
            ns_query_res,
            client
                .get(&format!(
                    "availability/block/hash/{}/namespace/{ns_id}",
                    header.commit()
                ))
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            ns_query_res,
            client
                .get(&format!(
                    "availability/block/payload-hash/{}/namespace/{ns_id}",
                    header.payload_commitment()
                ))
                .send()
                .await
                .unwrap()
        );

        // Verify namespace proof if present
        if let Some(ns_proof) = ns_query_res.proof {
            let vid_common: VidCommonQueryData<SeqTypes> = client
                .get(&format!("availability/vid/common/{block_num}"))
                .send()
                .await
                .unwrap();
            ns_proof
                .verify(
                    header.ns_table(),
                    &header.payload_commitment(),
                    vid_common.common(),
                )
                .unwrap();
        } else {
            // Namespace proof should be present if ns_id exists in ns_table
            assert!(header.ns_table().find_ns_id(&ns_id).is_none());
            assert!(ns_query_res.transactions.is_empty());
        }

        found_empty_block = found_empty_block || ns_query_res.transactions.is_empty();

        for txn in ns_query_res.transactions {
            if txn.commit() == hash {
                // Ensure that we validate an inclusion proof
                found_txn = true;
            }
        }
    }
    assert!(found_txn);
    assert!(found_empty_block);

    // Test range query.
    let ns_proofs: Vec<NamespaceProofQueryData> = client
        .get(&format!(
            "availability/block/{block_height}/{}/namespace/{ns_id}",
            block_height2 + 1
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(ns_proofs.len(), block_height2 + 1 - block_height);
    assert_eq!(&ns_proofs[0].transactions, std::slice::from_ref(&txn));
    assert_eq!(
        &ns_proofs[ns_proofs.len() - 1].transactions,
        std::slice::from_ref(&txn2)
    );
    for proof in &ns_proofs[1..ns_proofs.len() - 1] {
        assert_eq!(proof.transactions, &[]);
    }
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub(crate) async fn catchup_test_with_query_module<D: TestableSequencerDataSource>(
    _d: PhantomData<D>,
) {
    let storage = D::create_storage().await;
    catchup_test_helper(|opt| D::options(&storage, opt)).await
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub async fn test_non_consecutive_decide_with_failing_event_consumer<D>(_d: PhantomData<D>)
where
    D: TestableSequencerDataSource + Debug + 'static,
{
    use hotshot_types::new_protocol::CoordinatorEvent;

    #[derive(Clone, Copy, Debug)]
    struct FailConsumer;

    #[async_trait]
    impl EventConsumer for FailConsumer {
        async fn handle_event(&self, _: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
            bail!("mock error injection");
        }
    }

    let (pubkey, privkey) = PubKey::generated_from_seed_indexed([0; 32], 1);

    let storage = D::create_storage().await;
    let persistence = D::persistence_options(&storage).create().await.unwrap();
    let data_source: Arc<StorageState<network::Memory, NoStorage, _>> =
        Arc::new(StorageState::new(
            D::create(D::persistence_options(&storage), Default::default(), false)
                .await
                .unwrap(),
            ApiState::new(future::pending()),
        ));

    // Create two non-consecutive leaf chains.
    let mut chain1 = vec![];

    let genesis = Leaf2::genesis(
        &Default::default(),
        &NodeState::mock(),
        TEST_VERSIONS.test.base,
    )
    .await;
    let payload = genesis.block_payload().unwrap();
    let payload_bytes_arc = payload.encode();

    let avidm_param = init_avidm_param(2).unwrap();
    let weights = vec![1u32; 2];

    let ns_table = parse_ns_table(payload.byte_len().as_usize(), &payload.ns_table().encode());
    let (payload_commitment, shares) =
        AvidMScheme::ns_disperse(&avidm_param, &weights, &payload_bytes_arc, ns_table).unwrap();

    let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
        proposal: QuorumProposal2::<SeqTypes> {
            block_header: genesis.block_header().clone(),
            view_number: ViewNumber::genesis(),
            justify_qc: QuorumCertificate2::genesis(
                &ValidatedState::default(),
                &NodeState::mock(),
                MOCK_SEQUENCER_VERSIONS,
            )
            .await,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            next_epoch_justify_qc: None,
            epoch: None,
            state_cert: None,
        },
    };
    let mut qc = QuorumCertificate2::genesis(
        &ValidatedState::default(),
        &NodeState::mock(),
        MOCK_SEQUENCER_VERSIONS,
    )
    .await;

    let mut justify_qc = qc.clone();
    for i in 0..5 {
        *quorum_proposal.proposal.block_header.height_mut() = i;
        quorum_proposal.proposal.view_number = ViewNumber::new(i);
        quorum_proposal.proposal.justify_qc = justify_qc;
        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);
        qc.view_number = leaf.view_number();
        qc.data.leaf_commit = Committable::commit(&leaf);
        justify_qc = qc.clone();
        chain1.push((leaf.clone(), CertificatePair::non_epoch_change(qc.clone())));

        // Include a quorum proposal for each leaf.
        let quorum_proposal_signature =
            PubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                .expect("Failed to sign quorum_proposal");
        persistence
            .append_quorum_proposal2(&Proposal {
                data: quorum_proposal.clone(),
                signature: quorum_proposal_signature,
                _pd: Default::default(),
            })
            .await
            .unwrap();

        // Include VID information for each leaf.
        let share: VidDisperseShare<SeqTypes> = AvidMDisperseShare {
            view_number: leaf.view_number(),
            payload_commitment,
            share: shares[0].clone(),
            recipient_key: pubkey,
            epoch: Some(EpochNumber::new(0)),
            target_epoch: Some(EpochNumber::new(0)),
            common: avidm_param.clone(),
        }
        .into();

        persistence
            .append_vid(&share.to_proposal(&privkey).unwrap())
            .await
            .unwrap();

        // Include payload information for each leaf.
        let block_payload_signature =
            PubKey::sign(&privkey, &payload_bytes_arc).expect("Failed to sign block payload");
        let da_proposal_inner = DaProposal2::<SeqTypes> {
            encoded_transactions: payload_bytes_arc.clone(),
            metadata: payload.ns_table().clone(),
            view_number: leaf.view_number(),
            epoch: Some(EpochNumber::new(0)),
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };
        let da_proposal = Proposal {
            data: da_proposal_inner,
            signature: block_payload_signature,
            _pd: Default::default(),
        };
        persistence
            .append_da2(&da_proposal, VidCommitment::V1(payload_commitment))
            .await
            .unwrap();
    }
    // Split into two chains.
    let mut chain2 = chain1.split_off(2);
    // Make non-consecutive (i.e. we skip a leaf).
    chain2.remove(0);

    // Decide 2 leaves, but fail in event processing.
    let leaf_chain = chain1
        .iter()
        .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
        .collect::<Vec<_>>();
    tracing::info!("decide with event handling failure");
    persistence
        .append_decided_leaves(
            ViewNumber::new(1),
            leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
            None,
            &FailConsumer,
        )
        .await
        .unwrap();

    // Now decide remaining leaves successfully. We should now process a decide event for all
    // the leaves.
    let consumer = ApiEventConsumer::from(data_source.clone());
    let leaf_chain = chain2
        .iter()
        .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
        .collect::<Vec<_>>();
    tracing::info!("decide successfully");
    persistence
        .append_decided_leaves(
            ViewNumber::new(4),
            leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
            None,
            &consumer,
        )
        .await
        .unwrap();

    // Check that the leaves were moved to archive storage, along with payload and VID
    // information.
    for (leaf, cert) in chain1.iter().chain(&chain2) {
        tracing::info!(height = leaf.height(), "check archive");
        let qd = data_source.get_leaf(leaf.height() as usize).await.await;
        let stored_leaf: Leaf2 = qd.leaf().clone();
        let stored_qc = qd.qc().clone();
        assert_eq!(&stored_leaf, leaf);
        assert_eq!(&stored_qc, cert.qc());

        data_source
            .get_block(leaf.height() as usize)
            .await
            .try_resolve()
            .ok()
            .unwrap();
        data_source
            .get_vid_common(leaf.height() as usize)
            .await
            .try_resolve()
            .ok()
            .unwrap();

        // Check that all data has been garbage collected for the decided views.
        assert!(
            persistence
                .load_da_proposal(leaf.view_number())
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            persistence
                .load_vid_share(leaf.view_number())
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            persistence
                .load_quorum_proposal(leaf.view_number())
                .await
                .is_err()
        );
    }

    // Check that data has _not_ been garbage collected for the missing view.
    assert!(
        persistence
            .load_da_proposal(ViewNumber::new(2))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        persistence
            .load_vid_share(ViewNumber::new(2))
            .await
            .unwrap()
            .is_some()
    );
    persistence
        .load_quorum_proposal(ViewNumber::new(2))
        .await
        .unwrap();
}

#[rstest_reuse::apply(testable_sequencer_data_source)]
pub async fn test_decide_missing_data<D>(_d: PhantomData<D>)
where
    D: TestableSequencerDataSource + Debug + 'static,
{
    use ark_serialize::CanonicalDeserialize;

    let storage = D::create_storage().await;
    let persistence = D::persistence_options(&storage).create().await.unwrap();
    let data_source: Arc<StorageState<network::Memory, NoStorage, _>> =
        Arc::new(StorageState::new(
            D::create(D::persistence_options(&storage), Default::default(), false)
                .await
                .unwrap(),
            ApiState::new(future::pending()),
        ));
    let consumer = ApiEventConsumer::from(data_source.clone());

    let mut qc = QuorumCertificate2::genesis(
        &ValidatedState::default(),
        &NodeState::mock(),
        MOCK_SEQUENCER_VERSIONS,
    )
    .await;
    let leaf = Leaf2::genesis(
        &ValidatedState::default(),
        &NodeState::mock(),
        TEST_VERSIONS.test.base,
    )
    .await;

    // Append the genesis leaf. We don't use this for the test, because the update function will
    // automatically fill in the missing data for genesis. We just append this to get into a
    // consistent state to then append the leaf from view 1, which will have missing data.
    tracing::info!(?leaf, ?qc, "decide genesis leaf");
    persistence
        .append_decided_leaves(
            leaf.view_number(),
            [(
                &leaf_info(leaf.clone()),
                CertificatePair::non_epoch_change(qc.clone()),
            )],
            None,
            &consumer,
        )
        .await
        .unwrap();

    // Create another leaf, with missing data. We have to use a different payload commitment,
    // otherwise the database will be able to combine the empty payload from the genesis block
    // with this header, and the payload will not actually be missing.
    let mut block_header = leaf.block_header().clone();
    *block_header.height_mut() += 1;
    *block_header.payload_commitment_mut() = VidCommitment::V1(
        CanonicalDeserialize::deserialize_uncompressed_unchecked([1u8; 32].as_slice()).unwrap(),
    );
    let qp = QuorumProposalWrapper {
        proposal: QuorumProposal2 {
            block_header,
            view_number: leaf.view_number() + 1,
            justify_qc: qc.clone(),
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            next_epoch_justify_qc: None,
            epoch: None,
            state_cert: None,
        },
    };

    let leaf = Leaf2::from_quorum_proposal(&qp);
    qc.view_number = leaf.view_number();
    qc.data.leaf_commit = Committable::commit(&leaf);

    // Decide a leaf without the corresponding payload or VID.
    tracing::info!(?leaf, ?qc, "append leaf 1");
    persistence
        .append_decided_leaves(
            leaf.view_number(),
            [(
                &leaf_info(leaf.clone()),
                CertificatePair::non_epoch_change(qc),
            )],
            None,
            &consumer,
        )
        .await
        .unwrap();

    // Check that we still processed the leaf.
    assert_eq!(leaf, data_source.get_leaf(1).await.await.leaf().clone());
    assert!(data_source.get_vid_common(1).await.is_pending());
    assert!(data_source.get_block(1).await.is_pending());
}

fn leaf_info(leaf: Leaf2) -> LeafInfo<SeqTypes> {
    LeafInfo {
        leaf,
        vid_share: None,
        state: Default::default(),
        delta: None,
        state_cert: None,
    }
}
