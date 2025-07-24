//! This module implements interfaces necessary to run legacy builder
//! in HotShot testing harness.

use std::{collections::HashMap, fmt::Display, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use builder_shared::testing::constants::TEST_PROTOCOL_MAX_BLOCK_SIZE;
use hotshot::types::SignatureKey;
use hotshot_testing::{
    block_builder::{BuilderTask, TestBuilderImplementation},
    test_builder::BuilderChange,
};
use hotshot_types::{data::ViewNumber, traits::node_implementation::NodeType};
use tagged_base64::TaggedBase64;
use tokio::spawn;
use url::Url;
use vbs::version::StaticVersion;

use crate::service::{BuilderConfig, GlobalState};

/// Testing configuration for legacy builder
struct TestLegacyBuilderConfig<Types>
where
    Types: NodeType,
{
    _marker: PhantomData<Types>,
}

impl<Types> Default for TestLegacyBuilderConfig<Types>
where
    Types: NodeType,
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// [`TestBuilderImplementation`] for legacy builder.
/// Passed as a generic parameter to [`TestRunner::run_test`], it is be used
/// to instantiate builder API and builder task.
struct LegacyBuilderImpl {}

#[async_trait]
impl<Types> TestBuilderImplementation<Types> for LegacyBuilderImpl
where
    Types: NodeType<View = ViewNumber>,
    Types::InstanceState: Default,
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    type Config = TestLegacyBuilderConfig<Types>;

    /// This is mostly boilerplate to instantiate and start [`ProxyGlobalState`] APIs and initial [`BuilderState`]'s event loop.
    /// [`BuilderTask`] it returns will be injected into consensus runtime by HotShot testing harness and
    /// will forward transactions from hotshot event stream to the builder.
    async fn start(
        num_nodes: usize,
        url: Url,
        _config: <Self as TestBuilderImplementation<Types>>::Config,
        _changes: HashMap<u64, BuilderChange>,
    ) -> Box<dyn BuilderTask<Types>> {
        let service = GlobalState::new(
            BuilderConfig::test(),
            Types::InstanceState::default(),
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            num_nodes,
        );

        // Create tide-disco app based on global state
        let app = Arc::clone(&service)
            .into_app()
            .expect("Failed to create builder tide-disco app");

        let url_clone = url.clone();

        spawn(app.serve(url_clone, StaticVersion::<0, 1> {}));

        // Return the global state as a task that will be later started
        // by the test harness with event stream from one of HS nodes
        Box::new(LegacyBuilderTask { service })
    }
}

/// Legacy builder task. Stores all the necessary information to run builder service
struct LegacyBuilderTask<Types>
where
    Types: NodeType,
{
    service: Arc<GlobalState<Types>>,
}

impl<Types> BuilderTask<Types> for LegacyBuilderTask<Types>
where
    Types: NodeType<View = ViewNumber>,
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    fn start(
        self: Box<Self>,
        stream: Box<
            dyn futures::prelude::Stream<Item = hotshot::types::Event<Types>>
                + std::marker::Unpin
                + Send
                + 'static,
        >,
    ) {
        self.service.start_event_loop(stream);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use builder_shared::testing::{
        generation::{self, TransactionGenerationConfig},
        run_test,
        validation::BuilderValidationConfig,
    };
    use hotshot_example_types::node_types::{MemoryImpl, TestTypes, TestVersions};
    use hotshot_macros::cross_tests;
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        overall_safety_task::OverallSafetyPropertiesDescription,
        test_builder::TestDescription,
    };

    use crate::testing::integration::LegacyBuilderImpl;

    #[tokio::test(flavor = "multi_thread")]
    #[tracing::instrument]
    #[ignore = "slow"]
    async fn basic_test() {
        let num_successful_views = 45;
        let min_txns_per_view = 5;

        let mut metadata = TestDescription {
            txn_description: hotshot_testing::txn_task::TxnTaskDescription::RoundRobinTimeBased(
                Duration::MAX,
            ),
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(60),
                },
            ),
            overall_safety_properties: OverallSafetyPropertiesDescription {
                num_successful_views,
                ..Default::default()
            },
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 0;

        run_test::<TestVersions, LegacyBuilderImpl>(
            metadata,
            BuilderValidationConfig {
                expected_txn_num: num_successful_views * min_txns_per_view,
            },
            TransactionGenerationConfig {
                strategy: generation::GenerationStrategy::Random {
                    min_per_view: min_txns_per_view,
                    max_per_view: 10,
                    min_tx_size: 128,
                    max_tx_size: 1280,
                },
                endpoints: vec![],
            },
        )
        .await;
    }

    cross_tests!(
        TestName: example_cross_test,
        Impls: [MemoryImpl],
        BuilderImpls: [LegacyBuilderImpl],
        Types: [TestTypes],
        Versions: [TestVersions],
        Ignore: true,
        Metadata: {
            let mut metadata = TestDescription {
                validate_transactions : hotshot_testing::test_builder::nonempty_block_threshold((90,100)),
                txn_description : hotshot_testing::txn_task::TxnTaskDescription::RoundRobinTimeBased(Duration::from_millis(10)),
                completion_task_description : CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                            TimeBasedCompletionTaskDescription {
                                duration: Duration::from_secs(120),
                            },
                ),
                overall_safety_properties: OverallSafetyPropertiesDescription {
                    num_successful_views: 50,
                    ..Default::default()
                },
                ..Default::default()
            };

            metadata.test_config.epoch_height = 0;

            metadata
        },
    );
}
