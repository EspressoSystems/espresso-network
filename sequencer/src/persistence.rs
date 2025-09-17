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
use espresso_types::v0_3::ChainConfig;

pub mod fs;
pub mod no_storage;
mod persistence_metrics;
pub mod sql;

#[async_trait]
pub trait ChainConfigPersistence: Sized + Send + Sync {
    async fn insert_chain_config(&mut self, chain_config: ChainConfig) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, marker::PhantomData, sync::Arc, time::Duration};

    use alloy::{
        network::EthereumWallet,
        primitives::{Address, U256},
        providers::{ext::AnvilApi, Provider, ProviderBuilder},
    };
    use anyhow::bail;
    use async_lock::{Mutex, RwLock};
    use async_trait::async_trait;
    use committable::{Commitment, Committable};
    use espresso_contract_deployer::{
        builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table,
        Contract, Contracts,
    };
    use espresso_types::{
        traits::{
            EventConsumer, EventsPersistenceRead, MembershipPersistence, NullEventConsumer,
            PersistenceOptions, SequencerPersistence,
        },
        v0_3::{Fetcher, Validator},
        Event, L1Client, L1ClientOptions, Leaf, Leaf2, NodeState, PubKey, SeqTypes,
        SequencerVersions, ValidatedState,
    };
    use futures::{future::join_all, StreamExt, TryStreamExt};
    use hotshot::{
        types::{BLSPubKey, SignatureKey},
        InitializerEpochInfo,
    };
    use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::{availability::BlockQueryData, testing::mocks::MockVersions};
    use hotshot_types::{
        data::{
            ns_table::parse_ns_table, vid_commitment, vid_disperse::VidDisperseShare2, DaProposal2,
            EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidDisperseShare,
            ViewNumber,
        },
        event::{EventType, HotShotAction, LeafInfo},
        light_client::StateKeyPair,
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
    use indexmap::IndexMap;
    use portpicker::pick_unused_port;
    use staking_cli::demo::{setup_stake_table_contract_for_test, DelegationConfig};
    use surf_disco::Client;
    use tide_disco::error::ServerError;
    use tokio::{spawn, time::sleep};
    use vbs::version::{StaticVersion, StaticVersionType, Version};

    use crate::{
        api::{
            test_helpers::{TestNetwork, TestNetworkConfigBuilder, STAKE_TABLE_CAPACITY_FOR_TEST},
            Options,
        },
        catchup::NullStateCatchup,
        testing::{staking_priv_keys, TestConfigBuilder},
        SequencerApiVersion, RECENT_STAKE_TABLES_LIMIT,
    };

    #[async_trait]
    pub trait TestablePersistence: SequencerPersistence + MembershipPersistence {
        type Storage: Sync;

        async fn tmp_storage() -> Self::Storage;
        fn options(storage: &Self::Storage) -> impl PersistenceOptions<Persistence = Self>;

        async fn connect(storage: &Self::Storage) -> Self {
            Self::options(storage).create().await.unwrap()
        }
    }

    #[rstest_reuse::template]
    #[rstest::rstest]
    #[case(PhantomData::<crate::persistence::sql::Persistence>)]
    #[case(PhantomData::<crate::persistence::fs::Persistence>)]
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    pub fn persistence_types<P: TestablePersistence>(#[case] _p: PhantomData<P>) {}

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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_voted_view<P: TestablePersistence>(_p: PhantomData<P>) {
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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_restart_view<P: TestablePersistence>(_p: PhantomData<P>) {
        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Initially, there is no saved view.
        assert_eq!(storage.load_restart_view().await.unwrap(), None);

        // Store a view.
        let view1 = ViewNumber::genesis();
        storage
            .record_action(view1, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_restart_view().await.unwrap().unwrap(),
            view1 + 1
        );

        // Store a newer view, make sure storage gets updated.
        let view2 = view1 + 1;
        storage
            .record_action(view2, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_restart_view().await.unwrap().unwrap(),
            view2 + 1
        );

        // Store an old view, make sure storage is unchanged.
        storage
            .record_action(view1, None, HotShotAction::Vote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_restart_view().await.unwrap().unwrap(),
            view2 + 1
        );

        // store a higher proposed view, make sure storage is unchanged.
        storage
            .record_action(view2 + 1, None, HotShotAction::Propose)
            .await
            .unwrap();
        assert_eq!(
            storage.load_restart_view().await.unwrap().unwrap(),
            view2 + 1
        );

        // store a higher timeout vote view, make sure storage is unchanged.
        storage
            .record_action(view2 + 1, None, HotShotAction::TimeoutVote)
            .await
            .unwrap();
        assert_eq!(
            storage.load_restart_view().await.unwrap().unwrap(),
            view2 + 1
        );
    }

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_store_drb_input<P: TestablePersistence>(_p: PhantomData<P>) {
        use hotshot_types::drb::DrbInput;

        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;
        let difficulty_level = 10;

        // Initially, there is no saved info.
        if storage.load_drb_input(10).await.is_ok() {
            panic!("unexpected nonempty drb_input");
        }

        let drb_input_1 = DrbInput {
            epoch: 10,
            iteration: 10,
            value: [0u8; 32],
            difficulty_level,
        };

        let drb_input_2 = DrbInput {
            epoch: 10,
            iteration: 20,
            value: [0u8; 32],
            difficulty_level,
        };

        let drb_input_3 = DrbInput {
            epoch: 10,
            iteration: 30,
            value: [0u8; 32],
            difficulty_level,
        };

        let _ = storage.store_drb_input(drb_input_1.clone()).await;

        assert_eq!(storage.load_drb_input(10).await.unwrap(), drb_input_1);

        let _ = storage.store_drb_input(drb_input_3.clone()).await;

        // check that the drb input is overwritten
        assert_eq!(storage.load_drb_input(10).await.unwrap(), drb_input_3);

        let _ = storage.store_drb_input(drb_input_2.clone()).await;

        // check that the drb input is not overwritten by the older value
        assert_eq!(storage.load_drb_input(10).await.unwrap(), drb_input_3);
    }

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_epoch_info<P: TestablePersistence>(_p: PhantomData<P>) {
        let tmp = P::tmp_storage().await;
        let storage = P::connect(&tmp).await;

        // Initially, there is no saved info.
        assert_eq!(storage.load_start_epoch_info().await.unwrap(), Vec::new());

        // Store a drb result.
        storage
            .store_drb_result(EpochNumber::new(1), [1; 32])
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
            .store_drb_result(EpochNumber::new(2), [3; 32])
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
            .store_epoch_root(EpochNumber::new(1), header.clone())
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

        // Store more than the limit
        let total_epochs = RECENT_STAKE_TABLES_LIMIT + 10;
        for i in 0..total_epochs {
            let epoch = EpochNumber::new(i);
            let drb = [i as u8; 32];
            storage
                .store_drb_result(epoch, drb)
                .await
                .unwrap_or_else(|_| panic!("Failed to store DRB result for epoch {i}"));
        }

        let results = storage.load_start_epoch_info().await.unwrap();

        // Check that only the most recent RECENT_STAKE_TABLES_LIMIT epochs are returned
        assert_eq!(
            results.len(),
            RECENT_STAKE_TABLES_LIMIT as usize,
            "Should return only the most recent {RECENT_STAKE_TABLES_LIMIT} epochs",
        );

        for (i, info) in results.iter().enumerate() {
            let expected_epoch =
                EpochNumber::new(total_epochs - RECENT_STAKE_TABLES_LIMIT + i as u64);
            let expected_drb = [(total_epochs - RECENT_STAKE_TABLES_LIMIT + i as u64) as u8; 32];
            assert_eq!(info.epoch, expected_epoch, "invalid epoch at index {i}",);
            assert_eq!(info.drb_result, expected_drb, "invalid DRB at index {i}",);
            assert!(info.block_header.is_none(), "Expected no block header");
        }
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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_append_and_decide<P: TestablePersistence>(_p: PhantomData<P>) {
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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_upgrade_certificate<P: TestablePersistence>(_p: PhantomData<P>) {
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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_next_epoch_quorum_certificate<P: TestablePersistence>(_p: PhantomData<P>) {
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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_decide_with_failing_event_consumer<P: TestablePersistence>(
        _p: PhantomData<P>,
    ) {
        #[derive(Clone, Copy, Debug)]
        struct FailConsumer;

        #[async_trait]
        impl EventConsumer for FailConsumer {
            async fn handle_event(&self, _: &Event) -> anyhow::Result<()> {
                bail!("mock error injection");
            }
        }

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

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_pruning<P: TestablePersistence>(_p: PhantomData<P>) {
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

    async fn assert_events_eq<P: TestablePersistence>(
        persistence: &P,
        block: u64,
        stake_table_fetcher: &Fetcher,
        l1_client: &L1Client,
        stake_table_contract: Address,
    ) -> anyhow::Result<()> {
        // Load persisted events
        let (stored_l1, events) = persistence.load_events(block).await?;
        assert!(!events.is_empty());
        assert!(stored_l1.is_some());
        assert!(events.iter().all(|((l1_block, _), _)| *l1_block <= block));
        // Fetch events directly from the contract and compare with persisted data
        let contract_events = Fetcher::fetch_events_from_contract(
            l1_client.clone(),
            stake_table_contract,
            None,
            block,
        )
        .await
        .sort_events()?;
        assert_eq!(
            contract_events, events,
            "Events from contract and persistence do not match"
        );

        // Fetch events from stake table fetcher and compare with persisted data
        let fetched_events = stake_table_fetcher
            .fetch_events(stake_table_contract, block)
            .await?;
        assert_eq!(fetched_events, events);

        Ok(())
    }

    // test for validating stake table event fetching from persistence,
    // ensuring that persisted data matches the on-chain events and that event fetcher work correctly.
    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_stake_table_fetching_from_persistence<P: TestablePersistence>(
        #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
        stake_table_version: StakeTableContractVersion,
        _p: PhantomData<P>,
    ) -> anyhow::Result<()> {
        let epoch_height = 20;
        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let anvil_provider = network_config.anvil().unwrap();

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

        let persistence = persistence_options[0].clone().create().await.unwrap();

        // Build the config with PoS hook
        let l1_url = network_config.l1_url();

        let testnet_config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(query_api_options)
            .network_config(network_config.clone())
            .persistences(persistence_options.clone())
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, stake_table_version)
            .await
            .expect("Pos deployment failed")
            .build();

        //start the network
        let test_network = TestNetwork::new(testnet_config, PosVersion::new()).await;

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
        let membership_coordinator = test_network
            .server
            .consensus()
            .read()
            .await
            .membership_coordinator
            .clone();

        let l1_client = L1Client::new(vec![l1_url]).unwrap();
        let node_state = test_network.server.node_state();
        let chain_config = node_state.chain_config;
        let stake_table_contract = chain_config.stake_table_contract.unwrap();

        let current_membership = membership_coordinator.membership();
        {
            let membership_state = current_membership.read().await;
            let stake_table_fetcher = membership_state.fetcher();

            let block1 = anvil_provider
                .get_block_number()
                .await
                .expect("latest l1 block");

            assert_events_eq(
                &persistence,
                block1,
                stake_table_fetcher,
                &l1_client,
                stake_table_contract,
            )
            .await?;
        }
        let _epoch_4_blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(65)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let block2 = anvil_provider
            .get_block_number()
            .await
            .expect("latest l1 block");

        {
            let membership_state = current_membership.read().await;
            let stake_table_fetcher = membership_state.fetcher();

            assert_events_eq(
                &persistence,
                block2,
                stake_table_fetcher,
                &l1_client,
                stake_table_contract,
            )
            .await?;
        }
        Ok(())
    }

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_stake_table_background_fetching<P: TestablePersistence>(
        #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
        stake_table_version: StakeTableContractVersion,
        _p: PhantomData<P>,
    ) -> anyhow::Result<()> {
        use espresso_types::v0_3::ChainConfig;
        use hotshot_contract_adapter::stake_table::StakeTableContractVersion;

        let blocks_per_epoch = 10;

        let network_config = TestConfigBuilder::<1>::default()
            .epoch_height(blocks_per_epoch)
            .build();

        let anvil_provider = network_config.anvil().unwrap();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &network_config.hotshot_config().hotshot_stake_table(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let (_, priv_keys): (Vec<_>, Vec<_>) = (0..200)
            .map(|i| <PubKey as SignatureKey>::generated_from_seed_indexed([1; 32], i as u64))
            .unzip();
        let state_key_pairs = (0..200)
            .map(|i| StateKeyPair::generate_from_seed_indexed([2; 32], i as u64))
            .collect::<Vec<_>>();

        let validators = staking_priv_keys(&priv_keys, &state_key_pairs, 1000);

        let deployer = ProviderBuilder::new()
            .wallet(EthereumWallet::from(network_config.signer().clone()))
            .on_http(network_config.l1_url().clone());

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(deployer.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(blocks_per_epoch)
            .epoch_start_block(1)
            .exit_escrow_period(U256::from(blocks_per_epoch * 15 + 100))
            .multisig_pauser(network_config.signer().address())
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(U256::from(3590000000u64))
            .ops_timelock_delay(U256::from(0))
            .ops_timelock_admin(network_config.signer().address())
            .ops_timelock_proposers(vec![network_config.signer().address()])
            .ops_timelock_executors(vec![network_config.signer().address()])
            .safe_exit_timelock_delay(U256::from(10))
            .safe_exit_timelock_admin(network_config.signer().address())
            .safe_exit_timelock_proposers(vec![network_config.signer().address()])
            .safe_exit_timelock_executors(vec![network_config.signer().address()])
            .build()
            .unwrap();

        match stake_table_version {
            StakeTableContractVersion::V1 => args.deploy_to_stake_table_v1(&mut contracts).await,
            StakeTableContractVersion::V2 => args.deploy_all(&mut contracts).await,
        }
        .expect("contracts deployed");

        let st_addr = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy deployed");
        let l1_url = network_config.l1_url().clone();

        // new block every 1s
        anvil_provider
            .anvil_set_interval_mining(1)
            .await
            .expect("interval mining");

        // spawn a separate task
        // this is going to keep registering validators and multiple delegators
        // the interval mining is set to 1s so each transaction finalization would take atleast 1s
        spawn({
            let l1_url = l1_url.clone();
            async move {
                {
                    setup_stake_table_contract_for_test(
                        l1_url,
                        &deployer,
                        st_addr,
                        validators,
                        DelegationConfig::MultipleDelegators,
                    )
                    .await
                    .expect("stake table setup failed");
                }
            }
        });

        let storage = P::tmp_storage().await;
        let persistence = P::options(&storage).create().await.unwrap();

        let l1_client = L1ClientOptions {
            stake_table_update_interval: Duration::from_secs(7),
            l1_retry_delay: Duration::from_millis(10),
            l1_events_max_block_range: 10000,
            ..Default::default()
        }
        .connect(vec![l1_url])
        .unwrap();
        l1_client.spawn_tasks().await;

        let fetcher = Fetcher::new(
            Arc::new(NullStateCatchup::default()),
            Arc::new(Mutex::new(persistence.clone())),
            l1_client.clone(),
            ChainConfig {
                stake_table_contract: Some(st_addr),
                base_fee: 0.into(),
                ..Default::default()
            },
        );

        // sleep so that we have enough events
        sleep(Duration::from_secs(20)).await;

        fetcher.spawn_update_loop().await;
        let mut prev_l1_block = 0;
        let mut prev_events_len = 0;
        for _i in 0..10 {
            // Wait for more than update interval to assert that persistence was updated
            // L1 update interval is 7s in this test
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;

            let block = anvil_provider
                .get_block_number()
                .await
                .expect("latest l1 block");

            let (read_offset, persisted_events) = persistence.load_events(block).await?;
            let read_offset = read_offset.unwrap();
            let l1_block = match read_offset {
                EventsPersistenceRead::Complete => block,
                EventsPersistenceRead::UntilL1Block(block) => block,
            };

            tracing::info!("{l1_block:?}, persistence events = {persisted_events:?}.");
            assert!(persisted_events.len() > prev_events_len);

            assert!(l1_block > prev_l1_block, "events not updated");

            let contract_events =
                Fetcher::fetch_events_from_contract(l1_client.clone(), st_addr, None, l1_block)
                    .await
                    .sort_events()?;
            assert_eq!(persisted_events, contract_events);

            prev_l1_block = l1_block;
            prev_events_len = persisted_events.len();
        }

        Ok(())
    }

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_membership_persistence<P: TestablePersistence>(
        _p: PhantomData<P>,
    ) -> anyhow::Result<()> {
        let tmp = P::tmp_storage().await;
        let mut opt = P::options(&tmp);

        let storage = opt.create().await.unwrap();

        let validator = Validator::mock();
        let mut st = IndexMap::new();
        st.insert(validator.account, validator);

        storage
            .store_stake(EpochNumber::new(10), st.clone(), None, None)
            .await?;

        let (table, ..) = storage.load_stake(EpochNumber::new(10)).await?.unwrap();
        assert_eq!(st, table);

        let val2 = Validator::mock();
        let mut st2 = IndexMap::new();
        st2.insert(val2.account, val2);
        storage
            .store_stake(EpochNumber::new(11), st2.clone(), None, None)
            .await?;

        let tables = storage.load_latest_stake(4).await?.unwrap();
        let mut iter = tables.iter();
        assert_eq!(
            Some(&(EpochNumber::new(11), (st2.clone(), None), None)),
            iter.next()
        );
        assert_eq!(Some(&(EpochNumber::new(10), (st, None), None)), iter.next());
        assert_eq!(None, iter.next());

        for i in 0..=20 {
            storage
                .store_stake(EpochNumber::new(i), st2.clone(), None, None)
                .await?;
        }

        let tables = storage.load_latest_stake(5).await?.unwrap();
        let mut iter = tables.iter();
        assert_eq!(
            Some(&(EpochNumber::new(20), (st2.clone(), None), None)),
            iter.next()
        );
        assert_eq!(
            Some(&(EpochNumber::new(19), (st2.clone(), None), None)),
            iter.next()
        );
        assert_eq!(
            Some(&(EpochNumber::new(18), (st2.clone(), None), None)),
            iter.next()
        );
        assert_eq!(
            Some(&(EpochNumber::new(17), (st2.clone(), None), None)),
            iter.next()
        );
        assert_eq!(
            Some(&(EpochNumber::new(16), (st2, None), None)),
            iter.next()
        );
        assert_eq!(None, iter.next());

        Ok(())
    }

    #[rstest_reuse::apply(persistence_types)]
    pub async fn test_store_and_load_all_validators<P: TestablePersistence>(
        _p: PhantomData<P>,
    ) -> anyhow::Result<()> {
        let tmp = P::tmp_storage().await;
        let mut opt = P::options(&tmp);
        let storage = opt.create().await.unwrap();

        let mut vmap1 = IndexMap::new();
        for _i in 0..25 {
            let v = Validator::mock();
            vmap1.insert(v.account, v);
        }
        storage
            .store_all_validators(EpochNumber::new(10), vmap1.clone())
            .await?;

        let mut expected_all: Vec<_> = vmap1.clone().into_values().collect();
        expected_all.sort_by_key(|v| v.account);

        // Load all
        let loaded_all = storage
            .load_all_validators(EpochNumber::new(10), 0, 100)
            .await?;
        assert_eq!(expected_all, loaded_all);

        // Load first 10
        let loaded_first_10 = storage
            .load_all_validators(EpochNumber::new(10), 0, 10)
            .await?;
        assert_eq!(expected_all[..10], loaded_first_10);

        // Load next 10
        let loaded_next_10 = storage
            .load_all_validators(EpochNumber::new(10), 10, 10)
            .await?;
        assert_eq!(expected_all[10..20], loaded_next_10);

        // Load remaining 5
        let loaded_last_5 = storage
            .load_all_validators(EpochNumber::new(10), 20, 10)
            .await?;
        assert_eq!(expected_all[20..], loaded_last_5);

        // offset beyond size should return empty
        let loaded_empty = storage
            .load_all_validators(EpochNumber::new(10), 100, 10)
            .await?;
        assert!(loaded_empty.is_empty());

        // epoch 11
        let validator2 = Validator::mock();
        let mut vmap2 = IndexMap::new();
        vmap2.insert(validator2.account, validator2.clone());

        storage
            .store_all_validators(EpochNumber::new(11), vmap2.clone())
            .await?;

        let mut expected_epoch11: Vec<_> = vmap2.clone().into_values().collect();
        expected_epoch11.sort_by_key(|v| v.account);

        let loaded2 = storage
            .load_all_validators(EpochNumber::new(11), 0, 100)
            .await?;
        assert_eq!(expected_epoch11, loaded2);

        // Epoch 10 still there
        let loaded1_again = storage
            .load_all_validators(EpochNumber::new(10), 0, 100)
            .await?;
        assert_eq!(expected_all, loaded1_again);

        Ok(())
    }
}
