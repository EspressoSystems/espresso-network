//! Sequencer node persistence.
//!
//! This module implements the persistence required for a sequencer node to rejoin the network and
//! resume participating in consensus, in the event that its process crashes or is killed and loses
//! all in-memory state.
//!
//! This is distinct from the query service persistent storage found in the `api` module, which is
//! an extension that node operators can opt into. This module defines the minimum level of
//! persistence which is _required_ to run a node.

use async_trait::async_trait;
use espresso_types::v0_99::ChainConfig;

pub mod fs;
pub mod no_storage;
pub mod sql;

#[async_trait]
pub trait ChainConfigPersistence: Sized + Send + Sync {
    async fn insert_chain_config(&mut self, chain_config: ChainConfig) -> anyhow::Result<()>;
}

#[cfg(any(test, feature = "testing"))]
mod testing {

    use espresso_types::{
        traits::MembershipPersistence,
        v0::traits::{PersistenceOptions, SequencerPersistence},
    };

    use super::*;
    #[allow(dead_code)]
    #[async_trait]
    pub trait TestablePersistence: SequencerPersistence + MembershipPersistence {
        type Storage: Sync;

        async fn tmp_storage() -> Self::Storage;
        fn options(storage: &Self::Storage) -> impl PersistenceOptions<Persistence = Self>;

        async fn connect(storage: &Self::Storage) -> Self {
            Self::options(storage).create().await.unwrap()
        }
    }
}

#[cfg(test)]
#[espresso_macros::generic_tests]
mod persistence_tests {
    use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

    use alloy::{node_bindings::Anvil, signers::local::LocalSigner};
    use anyhow::bail;
    use async_lock::RwLock;
    use committable::{Commitment, Committable};
    use espresso_types::{
        traits::{EventConsumer, NullEventConsumer, PersistenceOptions},
        v0_3::StakeTableFetcher,
        Event, L1Client, Leaf, Leaf2, NodeState, PubKey, SeqTypes, SequencerVersions,
        ValidatedState,
    };
    use futures::{future::join_all, StreamExt, TryStreamExt};
    use hotshot::{
        types::{BLSPubKey, SignatureKey},
        InitializerEpochInfo,
    };
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::{availability::BlockQueryData, testing::mocks::MockVersions};
    use hotshot_types::{
        data::{
            ns_table::parse_ns_table, vid_commitment, vid_disperse::VidDisperseShare2, DaProposal2,
            EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidDisperseShare,
            ViewNumber,
        },
        event::{EventType, HotShotAction, LeafInfo},
        message::{convert_proposal, Proposal, UpgradeLock},
        simple_certificate::{
            NextEpochQuorumCertificate2, QuorumCertificate, QuorumCertificate2, UpgradeCertificate,
        },
        simple_vote::{NextEpochQuorumData2, QuorumData2, UpgradeProposalData, VersionedVoteData},
        traits::{
            block_contents::BlockHeader,
            node_implementation::{ConsensusTime, Versions},
            EncodeBytes,
        },
        utils::EpochTransitionIndicator,
        vid::avidm::{init_avidm_param, AvidMScheme},
        vote::HasViewNumber,
    };
    use portpicker::pick_unused_port;
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::Client;
    use testing::TestablePersistence;
    use tide_disco::error::ServerError;
    use vbs::version::{StaticVersion, StaticVersionType, Version};

    use super::*;
    use crate::{
        api::{
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        testing::TestConfigBuilder,
        SequencerApiVersion,
    };

    #[derive(Clone, Debug, Default)]
    struct EventCollector {
        events: Arc<RwLock<Vec<Event>>>,
    }

    impl EventCollector {
        async fn leaf_chain(&self) -> Vec<LeafInfo<SeqTypes>> {
            self.events
                .read()
                .await
                .iter()
                .flat_map(|event| {
                    let EventType::Decide { leaf_chain, .. } = &event.event else {
                        panic!("expected decide event, got {event:?}");
                    };
                    leaf_chain.iter().cloned().rev()
                })
                .collect::<Vec<_>>()
        }
    }

    #[async_trait]
    impl EventConsumer for EventCollector {
        async fn handle_event(&self, event: &Event) -> anyhow::Result<()> {
            self.events.write().await.push(event.clone());
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_voted_view<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Initially, there is no saved view.
        assert_eq!(storage.load_latest_acted_view().await.unwrap(), None);

        // Store a view.
        let view1 = ViewNumber::genesis();
        storage
            .record_action(view1, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_latest_acted_view().await.unwrap().unwrap(),
            view1
        );

        // Store a newer view, make sure storage gets updated.
        let view2 = view1 + 1;
        storage
            .record_action(view2, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_latest_acted_view().await.unwrap().unwrap(),
            view2
        );

        // Store an old view, make sure storage is unchanged.
        storage
            .record_action(view1, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_latest_acted_view().await.unwrap().unwrap(),
            view2
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_epoch_info<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Initially, there is no saved info.
        assert_eq!(storage.load_start_epoch_info().await.unwrap(), Vec::new());

        // Store a drb result.
        storage
            .add_drb_result(EpochNumber::new(1), [1; 32])
            .await
            .unwrap();
        assert_eq!(
            storage.load_start_epoch_info().await.unwrap(),
            vec![InitializerEpochInfo::<SeqTypes> {
                epoch: EpochNumber::new(1),
                drb_result: [1; 32],
                block_header: None,
            }]
        );

        // Store a second DRB result
        storage
            .add_drb_result(EpochNumber::new(2), [3; 32])
            .await
            .unwrap();
        assert_eq!(
            storage.load_start_epoch_info().await.unwrap(),
            vec![
                InitializerEpochInfo::<SeqTypes> {
                    epoch: EpochNumber::new(1),
                    drb_result: [1; 32],
                    block_header: None,
                },
                InitializerEpochInfo::<SeqTypes> {
                    epoch: EpochNumber::new(2),
                    drb_result: [3; 32],
                    block_header: None,
                }
            ]
        );

        // Make a header
        let instance_state = NodeState::mock();
        let validated_state = hotshot_types::traits::ValidatedState::genesis(&instance_state).0;
        let leaf: Leaf2 = Leaf::genesis::<MockVersions>(&validated_state, &instance_state)
            .await
            .into();
        let header = leaf.block_header().clone();

        // Test storing the header
        storage
            .add_epoch_root(EpochNumber::new(1), header.clone())
            .await
            .unwrap();
        assert_eq!(
            storage.load_start_epoch_info().await.unwrap(),
            vec![
                InitializerEpochInfo::<SeqTypes> {
                    epoch: EpochNumber::new(1),
                    drb_result: [1; 32],
                    block_header: Some(header.clone()),
                },
                InitializerEpochInfo::<SeqTypes> {
                    epoch: EpochNumber::new(2),
                    drb_result: [3; 32],
                    block_header: None,
                }
            ]
        );
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

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_append_and_decide<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Test append VID
        assert_eq!(
            storage.load_vid_share(ViewNumber::new(0)).await.unwrap(),
            None
        );

        let leaf: Leaf2 =
            Leaf2::genesis::<TestVersions>(&ValidatedState::default(), &NodeState::mock()).await;
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes_arc = leaf_payload.encode();

        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];

        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &leaf_payload_bytes_arc, ns_table)
                .unwrap();

        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let mut vid = VidDisperseShare2::<SeqTypes> {
            view_number: ViewNumber::new(0),
            payload_commitment,
            share: shares[0].clone(),
            recipient_key: pubkey,
            epoch: Some(EpochNumber::new(0)),
            target_epoch: Some(EpochNumber::new(0)),
            common: avidm_param,
        };
        let mut quorum_proposal = Proposal {
            data: QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: None,
                    block_header: leaf.block_header().clone(),
                    view_number: ViewNumber::genesis(),
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &ValidatedState::default(),
                        &NodeState::mock(),
                    )
                    .await,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_drb_result: None,
                    next_epoch_justify_qc: None,
                    state_cert: None,
                },
            },
            signature,
            _pd: Default::default(),
        };

        let vid_share0 = vid.clone().to_proposal(&privkey).unwrap().clone();

        storage.append_vid2(&vid_share0).await.unwrap();

        assert_eq!(
            storage.load_vid_share(ViewNumber::new(0)).await.unwrap(),
            Some(convert_proposal(vid_share0.clone()))
        );

        vid.view_number = ViewNumber::new(1);

        let vid_share1 = vid.clone().to_proposal(&privkey).unwrap().clone();
        storage.append_vid2(&vid_share1).await.unwrap();

        assert_eq!(
            storage.load_vid_share(vid.view_number()).await.unwrap(),
            Some(convert_proposal(vid_share1.clone()))
        );

        vid.view_number = ViewNumber::new(2);

        let vid_share2 = vid.clone().to_proposal(&privkey).unwrap().clone();
        storage.append_vid2(&vid_share2).await.unwrap();

        assert_eq!(
            storage.load_vid_share(vid.view_number()).await.unwrap(),
            Some(convert_proposal(vid_share2.clone()))
        );

        vid.view_number = ViewNumber::new(3);

        let vid_share3 = vid.clone().to_proposal(&privkey).unwrap().clone();
        storage.append_vid2(&vid_share3).await.unwrap();

        assert_eq!(
            storage.load_vid_share(vid.view_number()).await.unwrap(),
            Some(convert_proposal(vid_share3.clone()))
        );

        let block_payload_signature = BLSPubKey::sign(&privkey, &leaf_payload_bytes_arc)
            .expect("Failed to sign block payload");

        let da_proposal_inner = DaProposal2::<SeqTypes> {
            encoded_transactions: leaf_payload_bytes_arc.clone(),
            metadata: leaf_payload.ns_table().clone(),
            view_number: ViewNumber::new(0),
            epoch: None,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };

        let da_proposal = Proposal {
            data: da_proposal_inner,
            signature: block_payload_signature,
            _pd: Default::default(),
        };

        let vid_commitment = vid_commitment::<TestVersions>(
            &leaf_payload_bytes_arc,
            &leaf.block_header().metadata().encode(),
            2,
            <TestVersions as Versions>::Base::VERSION,
        );

        storage
            .append_da2(&da_proposal, vid_commitment)
            .await
            .unwrap();

        assert_eq!(
            storage.load_da_proposal(ViewNumber::new(0)).await.unwrap(),
            Some(da_proposal.clone())
        );

        let mut da_proposal1 = da_proposal.clone();
        da_proposal1.data.view_number = ViewNumber::new(1);
        storage
            .append_da2(&da_proposal1.clone(), vid_commitment)
            .await
            .unwrap();

        assert_eq!(
            storage
                .load_da_proposal(da_proposal1.data.view_number)
                .await
                .unwrap(),
            Some(da_proposal1.clone())
        );

        let mut da_proposal2 = da_proposal1.clone();
        da_proposal2.data.view_number = ViewNumber::new(2);
        storage
            .append_da2(&da_proposal2.clone(), vid_commitment)
            .await
            .unwrap();

        assert_eq!(
            storage
                .load_da_proposal(da_proposal2.data.view_number)
                .await
                .unwrap(),
            Some(da_proposal2.clone())
        );

        let mut da_proposal3 = da_proposal2.clone();
        da_proposal3.data.view_number = ViewNumber::new(3);
        storage
            .append_da2(&da_proposal3.clone(), vid_commitment)
            .await
            .unwrap();

        assert_eq!(
            storage
                .load_da_proposal(da_proposal3.data.view_number)
                .await
                .unwrap(),
            Some(da_proposal3.clone())
        );

        let quorum_proposal1 = quorum_proposal.clone();

        storage
            .append_quorum_proposal2(&quorum_proposal1)
            .await
            .unwrap();

        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            BTreeMap::from_iter([(ViewNumber::genesis(), quorum_proposal1.clone())])
        );

        quorum_proposal.data.proposal.view_number = ViewNumber::new(1);
        let quorum_proposal2 = quorum_proposal.clone();
        storage
            .append_quorum_proposal2(&quorum_proposal2)
            .await
            .unwrap();

        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            BTreeMap::from_iter([
                (ViewNumber::genesis(), quorum_proposal1.clone()),
                (ViewNumber::new(1), quorum_proposal2.clone())
            ])
        );

        quorum_proposal.data.proposal.view_number = ViewNumber::new(2);
        quorum_proposal.data.proposal.justify_qc.view_number = ViewNumber::new(1);
        let quorum_proposal3 = quorum_proposal.clone();
        storage
            .append_quorum_proposal2(&quorum_proposal3)
            .await
            .unwrap();

        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            BTreeMap::from_iter([
                (ViewNumber::genesis(), quorum_proposal1.clone()),
                (ViewNumber::new(1), quorum_proposal2.clone()),
                (ViewNumber::new(2), quorum_proposal3.clone())
            ])
        );

        quorum_proposal.data.proposal.view_number = ViewNumber::new(3);
        quorum_proposal.data.proposal.justify_qc.view_number = ViewNumber::new(2);

        // This one should stick around after GC runs.
        let quorum_proposal4 = quorum_proposal.clone();
        storage
            .append_quorum_proposal2(&quorum_proposal4)
            .await
            .unwrap();

        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            BTreeMap::from_iter([
                (ViewNumber::genesis(), quorum_proposal1.clone()),
                (ViewNumber::new(1), quorum_proposal2.clone()),
                (ViewNumber::new(2), quorum_proposal3.clone()),
                (ViewNumber::new(3), quorum_proposal4.clone())
            ])
        );

        // Test decide and garbage collection. Pass in a leaf chain with no VID shares or payloads,
        // so we have to fetch the missing data from storage.
        let leaves = [
            Leaf2::from_quorum_proposal(&quorum_proposal1.data),
            Leaf2::from_quorum_proposal(&quorum_proposal2.data),
            Leaf2::from_quorum_proposal(&quorum_proposal3.data),
            Leaf2::from_quorum_proposal(&quorum_proposal4.data),
        ];
        let mut final_qc = leaves[3].justify_qc();
        final_qc.view_number += 1;
        final_qc.data.leaf_commit = Committable::commit(&leaf);
        let qcs = [
            leaves[1].justify_qc(),
            leaves[2].justify_qc(),
            leaves[3].justify_qc(),
            final_qc,
        ];

        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            ViewNumber::genesis()
        );

        let consumer = EventCollector::default();
        let leaf_chain = leaves
            .iter()
            .take(3)
            .map(|leaf| leaf_info(leaf.clone()))
            .zip(&qcs)
            .collect::<Vec<_>>();
        tracing::info!(?leaf_chain, "decide view 2");
        storage
            .append_decided_leaves(
                ViewNumber::new(2),
                leaf_chain.iter().map(|(leaf, qc)| (leaf, (*qc).clone())),
                &consumer,
            )
            .await
            .unwrap();
        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            ViewNumber::new(2)
        );

        for i in 0..=2 {
            assert_eq!(
                storage.load_da_proposal(ViewNumber::new(i)).await.unwrap(),
                None
            );

            assert_eq!(
                storage.load_vid_share(ViewNumber::new(i)).await.unwrap(),
                None
            );
        }

        assert_eq!(
            storage.load_da_proposal(ViewNumber::new(3)).await.unwrap(),
            Some(da_proposal3)
        );

        assert_eq!(
            storage.load_vid_share(ViewNumber::new(3)).await.unwrap(),
            Some(convert_proposal(vid_share3.clone()))
        );

        let proposals = storage.load_quorum_proposals().await.unwrap();
        assert_eq!(
            proposals,
            BTreeMap::from_iter([(ViewNumber::new(3), quorum_proposal4)])
        );

        // A decide event should have been processed.
        for (leaf, info) in leaves.iter().zip(consumer.leaf_chain().await.iter()) {
            assert_eq!(info.leaf, *leaf);
            let decided_vid_share = info.vid_share.as_ref().unwrap();
            let view_number = match decided_vid_share {
                VidDisperseShare::V0(share) => share.view_number,
                VidDisperseShare::V1(share) => share.view_number,
            };
            assert_eq!(view_number, leaf.view_number());
        }

        // The decided leaf should not have been garbage collected.
        assert_eq!(
            storage.load_anchor_leaf().await.unwrap(),
            Some((leaves[2].clone(), qcs[2].clone()))
        );
        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            leaves[2].view_number()
        );

        // Process a second decide event.
        let consumer = EventCollector::default();
        tracing::info!(leaf = ?leaves[3], qc = ?qcs[3], "decide view 3");
        storage
            .append_decided_leaves(
                ViewNumber::new(3),
                vec![(&leaf_info(leaves[3].clone()), qcs[3].clone())],
                &consumer,
            )
            .await
            .unwrap();
        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            ViewNumber::new(3)
        );

        // A decide event should have been processed.
        let events = consumer.events.read().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].view_number, ViewNumber::new(3));
        let EventType::Decide { qc, leaf_chain, .. } = &events[0].event else {
            panic!("expected decide event, got {:?}", events[0]);
        };
        assert_eq!(**qc, qcs[3]);
        assert_eq!(leaf_chain.len(), 1);
        let info = &leaf_chain[0];
        assert_eq!(info.leaf, leaves[3]);

        // The remaining data should have been GCed.
        assert_eq!(
            storage.load_da_proposal(ViewNumber::new(3)).await.unwrap(),
            None
        );

        assert_eq!(
            storage.load_vid_share(ViewNumber::new(3)).await.unwrap(),
            None
        );
        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            BTreeMap::new()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_upgrade_certificate<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Test get upgrade certificate
        assert_eq!(storage.load_upgrade_certificate().await.unwrap(), None);

        let upgrade_data = UpgradeProposalData {
            old_version: Version { major: 0, minor: 1 },
            new_version: Version { major: 1, minor: 0 },
            decide_by: ViewNumber::genesis(),
            new_version_hash: Default::default(),
            old_version_last_view: ViewNumber::genesis(),
            new_version_first_view: ViewNumber::genesis(),
        };

        let decide_upgrade_certificate = UpgradeCertificate::<SeqTypes>::new(
            upgrade_data.clone(),
            upgrade_data.commit(),
            ViewNumber::genesis(),
            Default::default(),
            Default::default(),
        );
        let res = storage
            .store_upgrade_certificate(Some(decide_upgrade_certificate.clone()))
            .await;
        assert!(res.is_ok());

        let res = storage.load_upgrade_certificate().await.unwrap();
        let view_number = res.unwrap().view_number;
        assert_eq!(view_number, ViewNumber::genesis());

        let new_view_number_for_certificate = ViewNumber::new(50);
        let mut new_upgrade_certificate = decide_upgrade_certificate.clone();
        new_upgrade_certificate.view_number = new_view_number_for_certificate;

        let res = storage
            .store_upgrade_certificate(Some(new_upgrade_certificate.clone()))
            .await;
        assert!(res.is_ok());

        let res = storage.load_upgrade_certificate().await.unwrap();
        let view_number = res.unwrap().view_number;
        assert_eq!(view_number, new_view_number_for_certificate);
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_next_epoch_quorum_certificate<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        //  test that next epoch qc2 does not exist
        assert_eq!(
            storage.load_next_epoch_quorum_certificate().await.unwrap(),
            None
        );

        let upgrade_lock = UpgradeLock::<SeqTypes, TestVersions>::new();

        let genesis_view = ViewNumber::genesis();

        let leaf =
            Leaf2::genesis::<TestVersions>(&ValidatedState::default(), &NodeState::default()).await;
        let data: NextEpochQuorumData2<SeqTypes> = QuorumData2 {
            leaf_commit: leaf.commit(),
            epoch: Some(EpochNumber::new(1)),
            block_number: Some(leaf.height()),
        }
        .into();

        let versioned_data =
            VersionedVoteData::new_infallible(data.clone(), genesis_view, &upgrade_lock).await;

        let bytes: [u8; 32] = versioned_data.commit().into();

        let next_epoch_qc = NextEpochQuorumCertificate2::new(
            data,
            Commitment::from_raw(bytes),
            genesis_view,
            None,
            PhantomData,
        );

        let res = storage
            .store_next_epoch_quorum_certificate(next_epoch_qc.clone())
            .await;
        assert!(res.is_ok());

        let res = storage.load_next_epoch_quorum_certificate().await.unwrap();
        let view_number = res.unwrap().view_number;
        assert_eq!(view_number, ViewNumber::genesis());

        let new_view_number_for_qc = ViewNumber::new(50);
        let mut new_qc = next_epoch_qc.clone();
        new_qc.view_number = new_view_number_for_qc;

        let res = storage
            .store_next_epoch_quorum_certificate(new_qc.clone())
            .await;
        assert!(res.is_ok());

        let res = storage.load_next_epoch_quorum_certificate().await.unwrap();
        let view_number = res.unwrap().view_number;
        assert_eq!(view_number, new_view_number_for_qc);
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_decide_with_failing_event_consumer<P: TestablePersistence>() {
        #[derive(Clone, Copy, Debug)]
        struct FailConsumer;

        #[async_trait]
        impl EventConsumer for FailConsumer {
            async fn handle_event(&self, _: &Event) -> anyhow::Result<()> {
                bail!("mock error injection");
            }
        }

        setup_test();

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Create a short blockchain.
        let mut chain = vec![];

        let leaf: Leaf2 =
            Leaf::genesis::<MockVersions>(&ValidatedState::default(), &NodeState::mock())
                .await
                .into();
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes_arc = leaf_payload.encode();
        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];
        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &leaf_payload_bytes_arc, ns_table)
                .unwrap();

        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let mut vid = VidDisperseShare2::<SeqTypes> {
            view_number: ViewNumber::new(0),
            payload_commitment,
            share: shares[0].clone(),
            recipient_key: pubkey,
            epoch: Some(EpochNumber::new(0)),
            target_epoch: Some(EpochNumber::new(0)),
            common: avidm_param,
        }
        .to_proposal(&privkey)
        .unwrap()
        .clone();
        let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                block_header: leaf.block_header().clone(),
                view_number: ViewNumber::genesis(),
                justify_qc: QuorumCertificate::genesis::<TestVersions>(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                )
                .await
                .to_qc2(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };
        let mut qc = QuorumCertificate2::genesis::<TestVersions>(
            &ValidatedState::default(),
            &NodeState::mock(),
        )
        .await;

        let block_payload_signature = BLSPubKey::sign(&privkey, &leaf_payload_bytes_arc)
            .expect("Failed to sign block payload");
        let mut da_proposal = Proposal {
            data: DaProposal2::<SeqTypes> {
                encoded_transactions: leaf_payload_bytes_arc.clone(),
                metadata: leaf_payload.ns_table().clone(),
                view_number: ViewNumber::new(0),
                epoch: Some(EpochNumber::new(0)),
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            },
            signature: block_payload_signature,
            _pd: Default::default(),
        };

        let vid_commitment = vid_commitment::<TestVersions>(
            &leaf_payload_bytes_arc,
            &leaf.block_header().metadata().encode(),
            2,
            <TestVersions as Versions>::Base::VERSION,
        );

        for i in 0..4 {
            quorum_proposal.proposal.view_number = ViewNumber::new(i);
            let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);
            qc.view_number = leaf.view_number();
            qc.data.leaf_commit = Committable::commit(&leaf);
            vid.data.view_number = leaf.view_number();
            da_proposal.data.view_number = leaf.view_number();
            chain.push((leaf.clone(), qc.clone(), vid.clone(), da_proposal.clone()));
        }

        // Add proposals.
        for (_, _, vid, da) in &chain {
            tracing::info!(?da, ?vid, "insert proposal");
            storage.append_da2(da, vid_commitment).await.unwrap();
            storage.append_vid2(vid).await.unwrap();
        }

        // Decide 2 leaves, but fail in event processing.
        let leaf_chain = chain
            .iter()
            .take(2)
            .map(|(leaf, qc, ..)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        tracing::info!("decide with event handling failure");
        storage
            .append_decided_leaves(
                ViewNumber::new(1),
                leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
                &FailConsumer,
            )
            .await
            .unwrap();
        // No garbage collection should have run.
        for i in 0..4 {
            tracing::info!(i, "check proposal availability");
            assert!(storage
                .load_vid_share(ViewNumber::new(i))
                .await
                .unwrap()
                .is_some());
            assert!(storage
                .load_da_proposal(ViewNumber::new(i))
                .await
                .unwrap()
                .is_some());
        }
        tracing::info!("check anchor leaf updated");
        assert_eq!(
            storage
                .load_anchor_leaf()
                .await
                .unwrap()
                .unwrap()
                .0
                .view_number(),
            ViewNumber::new(1)
        );
        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            ViewNumber::new(1)
        );

        // Now decide remaining leaves successfully. We should now garbage collect and process a
        // decide event for all the leaves.
        let consumer = EventCollector::default();
        let leaf_chain = chain
            .iter()
            .skip(2)
            .map(|(leaf, qc, ..)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        tracing::info!("decide successfully");
        storage
            .append_decided_leaves(
                ViewNumber::new(3),
                leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
                &consumer,
            )
            .await
            .unwrap();
        // Garbage collection should have run.
        for i in 0..4 {
            tracing::info!(i, "check proposal garbage collected");
            assert!(storage
                .load_vid_share(ViewNumber::new(i))
                .await
                .unwrap()
                .is_none());
            assert!(storage
                .load_da_proposal(ViewNumber::new(i))
                .await
                .unwrap()
                .is_none());
        }
        tracing::info!("check anchor leaf updated");
        assert_eq!(
            storage
                .load_anchor_leaf()
                .await
                .unwrap()
                .unwrap()
                .0
                .view_number(),
            ViewNumber::new(3)
        );
        assert_eq!(
            storage.load_anchor_view().await.unwrap(),
            ViewNumber::new(3)
        );

        // Check decide event.
        tracing::info!("check decide event");
        let leaf_chain = consumer.leaf_chain().await;
        assert_eq!(leaf_chain.len(), 4, "{leaf_chain:#?}");
        for ((leaf, ..), info) in chain.iter().zip(leaf_chain.iter()) {
            assert_eq!(info.leaf, *leaf);
            let decided_vid_share = info.vid_share.as_ref().unwrap();
            let view_number = match decided_vid_share {
                VidDisperseShare::V0(share) => share.view_number,
                VidDisperseShare::V1(share) => share.view_number,
            };
            assert_eq!(view_number, leaf.view_number());
            assert!(info.leaf.block_payload().is_some());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_pruning<P: TestablePersistence>() {
        setup_test();

        let tmp = P::tmp_storage().await;

        let mut options = P::options(&tmp);
        options.set_view_retention(1);
        let storage = options.create().await.unwrap();

        // Add some "old" data, from view 0.
        let leaf =
            Leaf::genesis::<MockVersions>(&ValidatedState::default(), &NodeState::mock()).await;
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes_arc = leaf_payload.encode();
        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];

        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &leaf_payload_bytes_arc, ns_table)
                .unwrap();

        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let vid_share = VidDisperseShare2::<SeqTypes> {
            view_number: ViewNumber::new(0),
            payload_commitment,
            share: shares[0].clone(),
            recipient_key: pubkey,
            epoch: None,
            target_epoch: None,
            common: avidm_param,
        }
        .to_proposal(&privkey)
        .unwrap()
        .clone();

        let quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                block_header: leaf.block_header().clone(),
                view_number: ViewNumber::genesis(),
                justify_qc: QuorumCertificate::genesis::<TestVersions>(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                )
                .await
                .to_qc2(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };
        let quorum_proposal_signature =
            BLSPubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                .expect("Failed to sign quorum proposal");
        let quorum_proposal = Proposal {
            data: quorum_proposal,
            signature: quorum_proposal_signature,
            _pd: Default::default(),
        };

        let block_payload_signature = BLSPubKey::sign(&privkey, &leaf_payload_bytes_arc)
            .expect("Failed to sign block payload");
        let da_proposal = Proposal {
            data: DaProposal2::<SeqTypes> {
                encoded_transactions: leaf_payload_bytes_arc,
                metadata: leaf_payload.ns_table().clone(),
                view_number: ViewNumber::new(0),
                epoch: None,
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            },
            signature: block_payload_signature,
            _pd: Default::default(),
        };

        storage
            .append_da2(&da_proposal, VidCommitment::V1(payload_commitment))
            .await
            .unwrap();
        storage.append_vid2(&vid_share).await.unwrap();
        storage
            .append_quorum_proposal2(&quorum_proposal)
            .await
            .unwrap();

        // Decide a newer view, view 1.
        storage
            .append_decided_leaves(ViewNumber::new(1), [], &NullEventConsumer)
            .await
            .unwrap();

        // The old data is not more than the retention period (1 view) old, so it should not be
        // GCed.
        assert_eq!(
            storage
                .load_da_proposal(ViewNumber::new(0))
                .await
                .unwrap()
                .unwrap(),
            da_proposal
        );
        assert_eq!(
            storage
                .load_vid_share(ViewNumber::new(0))
                .await
                .unwrap()
                .unwrap(),
            convert_proposal(vid_share)
        );
        assert_eq!(
            storage
                .load_quorum_proposal(ViewNumber::new(0))
                .await
                .unwrap(),
            quorum_proposal
        );

        // Decide an even newer view, triggering GC of the old data.
        storage
            .append_decided_leaves(ViewNumber::new(2), [], &NullEventConsumer)
            .await
            .unwrap();
        assert!(storage
            .load_da_proposal(ViewNumber::new(0))
            .await
            .unwrap()
            .is_none());
        assert!(storage
            .load_vid_share(ViewNumber::new(0))
            .await
            .unwrap()
            .is_none());
        assert!(storage
            .load_quorum_proposal(ViewNumber::new(0))
            .await
            .is_err());
    }

    // test for validating stake table event fetching from persistence,
    // ensuring that persisted data matches the on-chain events and that event fetcher work correctly.
    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_stake_table_fetching_from_persistence<P: TestablePersistence>(
    ) -> anyhow::Result<()> {
        setup_test();

        let epoch_height = 20;
        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let anvil_instance = Anvil::new().args(["--slots-in-an-epoch", "0"]).spawn();
        let l1_rpc_url = anvil_instance.endpoint_url();
        let l1_signer_key = anvil_instance.keys()[0].clone();
        let signer = LocalSigner::from(l1_signer_key);

        let network_config = TestConfigBuilder::default()
            .l1_url(l1_rpc_url.clone())
            .signer(signer.clone())
            .epoch_height(epoch_height)
            .build();

        let query_service_port = pick_unused_port().expect("No ports free for query service");
        let query_api_options = Options::with_port(query_service_port);

        const NODE_COUNT: usize = 2;

        let storage = join_all((0..NODE_COUNT).map(|_| P::tmp_storage())).await;
        let persistence_options: [_; NODE_COUNT] = storage
            .iter()
            .map(P::options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // Build the config with PoS hook

        let testnet_config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(query_api_options)
            .network_config(network_config.clone())
            .persistences(persistence_options.clone())
            .pos_hook::<PosVersion>(true)
            .await
            .expect("Pos deployment failed")
            .build();

        //start the network

        let mut test_network = TestNetwork::new(testnet_config, PosVersion::new()).await;

        let client: Client<ServerError, SequencerApiVersion> = Client::new(
            format!("http://localhost:{query_service_port}")
                .parse()
                .unwrap(),
        );
        client.connect(None).await;
        tracing::info!(query_service_port, "server running");

        // wait until we enter in epoch 3
        let _initial_blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(40)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // Load initial persisted events and validate they exist.
        let persistence = persistence_options[0].clone().create().await.unwrap();
        let res1 = persistence
            .load_events()
            .await
            .expect("failed to load events");
        assert!(res1.is_some());

        let (events1_l1_block, events1) = res1.unwrap();
        assert!(!events1.is_empty());

        let _epoch_4_blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(65)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // Stop consensus to freeze the state
        test_network.stop_consensus().await;

        let latest_events_data = persistence
            .load_events()
            .await
            .expect("failed to load events");
        assert!(latest_events_data.is_some());

        let (latest_l1_block, final_persisted_events) = latest_events_data.unwrap();
        assert!(!final_persisted_events.is_empty());

        let membership_coordinator = test_network
            .server
            .consensus()
            .read()
            .await
            .membership_coordinator
            .clone();

        let l1_client = L1Client::new(vec![l1_rpc_url]).unwrap();
        let node_state = test_network.server.node_state();
        let chain_config = node_state.chain_config;
        let stake_table_contract = chain_config.stake_table_contract.unwrap();

        // Fetch events directly from the contract and compare with persisted data.
        let contract_events = StakeTableFetcher::fetch_events_from_contract(
            l1_client.clone(),
            stake_table_contract,
            None,
            latest_l1_block,
        )
        .await
        .unwrap()
        .sort_events()
        .unwrap();

        assert_eq!(
            contract_events, final_persisted_events,
            "Events from contract and persistence do not match"
        );

        let current_membership = membership_coordinator.membership();
        let membership_state = current_membership.read().await;
        let stake_table_fetcher = membership_state.fetcher();

        let fetched_events = stake_table_fetcher
            .fetch_events(stake_table_contract, latest_l1_block)
            .await
            .unwrap();
        assert_eq!(fetched_events, final_persisted_events);

        // store an old snapshot of events in contract
        // so that fetcher has to get additional events from the contract
        persistence
            .store_events(events1_l1_block, events1.clone())
            .await
            .unwrap();
        let events = persistence.load_events().await.unwrap();
        assert_eq!(events1.clone(), events.clone().unwrap().1);
        assert_eq!(events.unwrap().0, events1_l1_block);

        // Ensure fetcher events still matches contract state after restoring older snapshot.
        let fetcher_events = stake_table_fetcher
            .fetch_events(stake_table_contract, latest_l1_block)
            .await
            .unwrap();
        let expected_events = StakeTableFetcher::fetch_events_from_contract(
            l1_client,
            stake_table_contract,
            None,
            latest_l1_block,
        )
        .await
        .unwrap()
        .sort_events()
        .unwrap();

        assert_eq!(fetcher_events, expected_events);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_stake_table_background_fetching<P: TestablePersistence>() -> anyhow::Result<()>
    {
        setup_test();

        let epoch_height = 30;
        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let anvil_instance = Anvil::new()
            .args(["--slots-in-an-epoch", "0", "--block-time", "1"])
            .spawn();
        let l1_rpc_url = anvil_instance.endpoint_url();
        let l1_signer_key = anvil_instance.keys()[0].clone();
        let signer = LocalSigner::from(l1_signer_key);

        let network_config = TestConfigBuilder::default()
            .l1_url(l1_rpc_url.clone())
            .signer(signer.clone())
            .epoch_height(epoch_height)
            .build();

        let query_service_port = pick_unused_port().expect("No ports free for query service");
        let query_api_options = Options::with_port(query_service_port);

        const NODE_COUNT: usize = 2;

        let storage = join_all((0..NODE_COUNT).map(|_| P::tmp_storage())).await;
        let persistence_options: [_; NODE_COUNT] = storage
            .iter()
            .map(P::options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // Build the config with PoS hook
        let testnet_config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(query_api_options)
            .network_config(network_config.clone())
            .persistences(persistence_options.clone())
            .pos_hook::<PosVersion>(true)
            .await
            .expect("Pos deployment failed")
            .build();

        //start the network
        let _test_network = TestNetwork::new(testnet_config, PosVersion::new()).await;

        let client: Client<ServerError, SequencerApiVersion> = Client::new(
            format!("http://localhost:{query_service_port}")
                .parse()
                .unwrap(),
        );
        client.connect(None).await;
        tracing::info!(query_service_port, "server running");

        // wait until we enter in epoch 3
        let _initial_blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(60)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // Load initial persisted events and validate they exist.
        let persistence = persistence_options[0].clone().create().await.unwrap();
        let res1 = persistence
            .load_events()
            .await
            .expect("failed to load events");
        assert!(res1.is_some());

        let (mut prev_l1, prev_events) = res1.unwrap();
        assert!(!prev_events.is_empty());

        for _i in 0..10 {
            // Wait for more than update interval to assert that persistence was updated
            // L1 update interval is 5s for testing
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;

            let res = persistence
                .load_events()
                .await
                .expect("failed to load events");

            let (l1, events) = res.unwrap();
            assert!(!events.is_empty());

            assert!(l1 > prev_l1, "events not updated");
            prev_l1 = l1;
        }

        Ok(())
    }
}
