// use std::{collections::HashMap, rc::Rc, time::Duration};

// use hotshot_example_types::{
//     node_types::{Libp2pImpl, MarketplaceTestVersions, MemoryImpl, PushCdnImpl},
//     state_types::TestTypes,
// };
// use hotshot_macros::cross_tests;
// use hotshot_testing::{
//     block_builder::SimpleBuilderImplementation,
//     byzantine::byzantine_behaviour::ViewDelay,
//     completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
//     test_builder::{Behaviour, TestDescription},
// };
// use hotshot_types::{data::ViewNumber, traits::node_implementation::ConsensusTime};

// cross_tests!(
//     TestName: view_delay,
//     Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
//     Types: [TestTypes],
//     Versions: [MarketplaceTestVersions],
//     Ignore: false,
//     Metadata: {

//         let behaviour = Rc::new(|node_id| {
//                 let view_delay = ViewDelay {
//                     number_of_views_to_delay: node_id/3,
//                     events_for_view: HashMap::new(),
//                     stop_view_delay_at_view_number: 25,
//                 };
//                 match node_id {
//                     6|10|14 => Behaviour::Byzantine(Box::new(view_delay)),
//                     _ => Behaviour::Standard,
//                 }
//             });

//         let mut metadata = TestDescription {
//             // allow more time to pass in CI
//             completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
//                                              TimeBasedCompletionTaskDescription {
//                                                  duration: Duration::from_secs(60),
//                                              },
//                                          ),
//             behaviour,
//             epoch_height: 0,
//             ..TestDescription::default()
//         };

//         let num_nodes_with_stake = 15;
//         metadata.num_nodes_with_stake = num_nodes_with_stake;
//         metadata.da_staked_committee_size = num_nodes_with_stake;
//         metadata.overall_safety_properties.num_successful_views = 20;
//         metadata.overall_safety_properties.expected_view_failures = HashMap::from([
//             (ViewNumber::new(6), false),
//             (ViewNumber::new(10), false),
//             (ViewNumber::new(14), false),
//             (ViewNumber::new(21), false),
//             (ViewNumber::new(25), false),
//         ]);
//         metadata
//     },
// );
