use std::{collections::HashSet, rc::Rc, sync::Arc, time::Duration};

use async_lock::RwLock;
use hotshot_example_types::{
    node_types::{Libp2pImpl, MarketplaceTestVersions, MemoryImpl, PushCdnImpl, TestVersions},
    state_types::TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    byzantine::byzantine_behaviour::{
        BadProposalViewDos, DishonestDa, DishonestLeader, DishonestVoter, DishonestVoting,
        DoubleProposeVote,
    },
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    test_builder::{Behaviour, TestDescription},
};
use hotshot_types::{
    message::{GeneralConsensusMessage, MessageKind, SequencingMessage},
    traits::{election::Membership, network::TransmitType, node_implementation::NodeType},
    vote::HasViewNumber,
};

cross_tests!(
    TestName: double_propose_vote,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(|node_id| { match node_id {
          1 => Behaviour::Byzantine(Box::new(DoubleProposeVote)),
          _ => Behaviour::Standard,
          } });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            behaviour,
            ..TestDescription::default()
        };
        metadata.test_config.epoch_height = 0;

        metadata
    },
);

// Test where node 4 sends out the correct quorum proposal and additionally spams the network with an extra 99 malformed proposals
cross_tests!(
    TestName: multiple_bad_proposals,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(|node_id| { match node_id {
          4 => Behaviour::Byzantine(Box::new(BadProposalViewDos { multiplier: 100, increment: 1 })),
          _ => Behaviour::Standard,
          } });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(12,12);

        metadata.test_config.epoch_height = 0;
        metadata.overall_safety_properties.num_successful_views = 10;
        metadata.overall_safety_properties.expected_view_failures = vec![3, 4];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(12);
        metadata
    },
);

cross_tests!(
    TestName: dishonest_leader,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(|node_id| {
                let dishonest_leader = DishonestLeader {
                    dishonest_at_proposal_numbers: HashSet::from([2, 3]),
                    validated_proposals: Vec::new(),
                    total_proposals_from_node: 0,
                    view_look_back: 1,
                    dishonest_proposal_view_numbers: Arc::new(RwLock::new(HashSet::new())),
                };
                match node_id {
                    2 => Behaviour::Byzantine(Box::new(dishonest_leader)),
                    _ => Behaviour::Standard,
                }
            });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(5,5);

        metadata.test_config.epoch_height = 0;
        metadata.overall_safety_properties.expected_view_failures = vec![6, 7, 11, 12];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(20);

        metadata
    },
);

cross_tests!(
    TestName: dishonest_da,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(|node_id| {
                let dishonest_da = DishonestDa {
                    dishonest_at_da_cert_sent_numbers: HashSet::from([2]),
                    total_da_certs_sent_from_node: 0,
                    total_views_add_to_cert: 4
                };
                match node_id {
                    2 => Behaviour::Byzantine(Box::new(dishonest_da)),
                    _ => Behaviour::Standard,
                }
            });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(10,10);

        metadata.test_config.epoch_height = 0;
        metadata
    },
);

cross_tests!(
    TestName: dishonest_voting,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let nodes_count = 10;
        let behaviour = Rc::new(move |node_id| {
            let dishonest_voting = DishonestVoting {
                view_increment: nodes_count,
                modifier: Arc::new(move |_pk, message_kind, transmit_type: &mut TransmitType<TestTypes>, membership: &<TestTypes as NodeType>::Membership| {
                    if let MessageKind::Consensus(SequencingMessage::General(GeneralConsensusMessage::Vote(vote))) = message_kind {
                        *transmit_type = TransmitType::Direct(membership.leader(vote.view_number() + 1 - nodes_count, None).unwrap());
                    } else {
                        {}
                    }
                })
            };
            match node_id {
                5 => Behaviour::Byzantine(Box::new(dishonest_voting)),
                _ => Behaviour::Standard,
            }
        });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(nodes_count, nodes_count);

        metadata.test_config.epoch_height = 0;
        metadata
    },
);

cross_tests!(
    TestName: coordination_attack,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let dishonest_proposal_view_numbers = Arc::new(RwLock::new(HashSet::new()));
        let behaviour = Rc::new(move |node_id| {
            match node_id {
                4 => Behaviour::Byzantine(Box::new(DishonestLeader {
                    // On second proposal send a dishonest qc
                    dishonest_at_proposal_numbers: HashSet::from([2]),
                    validated_proposals: Vec::new(),
                    total_proposals_from_node: 0,
                    view_look_back: 1,
                    dishonest_proposal_view_numbers: Arc::clone(&dishonest_proposal_view_numbers),
                })),
                5 | 6 => Behaviour::Byzantine(Box::new(DishonestVoter {
                    votes_sent: Vec::new(),
                    dishonest_proposal_view_numbers: Arc::clone(&dishonest_proposal_view_numbers),
                })),
                _ => Behaviour::Standard,
            }
        });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(60),
                },
            ),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(10,10);

        metadata.test_config.epoch_height = 0;
        metadata.overall_safety_properties.expected_view_failures = vec![13, 14];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(12);
        metadata
    },
);
