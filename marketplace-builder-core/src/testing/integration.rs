//! This module implements interfaces necessary to run marketplace builder
//! in HotShot testing harness.

use std::{collections::HashMap, fmt::Display, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
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

use crate::{
    hooks::{BuilderHooks, NoHooks},
    service::{BuilderConfig, GlobalState},
};

/// Testing configuration for marketplace builder
/// Stores hooks that will be used in the builder in a type-erased manner,
/// allowing for runtime configuration of hooks to use in tests.
struct TestMarketplaceBuilderConfig<Types>
where
    Types: NodeType,
{
    hooks: Box<dyn BuilderHooks<Types>>,
}

impl<Types> Default for TestMarketplaceBuilderConfig<Types>
where
    Types: NodeType,
{
    fn default() -> Self {
        Self {
            hooks: Box::new(NoHooks(PhantomData)),
        }
    }
}

/// [`TestBuilderImplementation`] for marketplace builder.
/// Passed as a generic parameter to [`TestRunner::run_test`], it is be used
/// to instantiate builder API and builder task.
struct MarketplaceBuilderImpl {}

#[async_trait]
impl<Types> TestBuilderImplementation<Types> for MarketplaceBuilderImpl
where
    Types: NodeType<View = ViewNumber>,
    Types::InstanceState: Default,
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    type Config = TestMarketplaceBuilderConfig<Types>;

    /// This is mostly boilerplate to instantiate and start [`ProxyGlobalState`] APIs and initial [`BuilderState`]'s event loop.
    /// [`BuilderTask`] it returns will be injected into consensus runtime by HotShot testing harness and
    /// will forward transactions from hotshot event stream to the builder.
    async fn start(
        _n_nodes: usize,
        url: Url,
        config: Self::Config,
        _changes: HashMap<u64, BuilderChange>,
    ) -> Box<dyn BuilderTask<Types>> {
        // Create the global state
        let service = GlobalState::new(BuilderConfig::test(), config.hooks);

        // Create tide-disco app based on global state
        let app = Arc::clone(&service)
            .into_app()
            .expect("Failed to create builder tide-disco app");

        let url_clone = url.clone();

        spawn(app.serve(url_clone, StaticVersion::<0, 1> {}));

        // Return the global state as a task that will be later started
        // by the test harness with event stream from one of HS nodes
        Box::new(MarketplaceBuilderTask { service })
    }
}

/// Marketplace builder task. Stores all the necessary information to run builder service
struct MarketplaceBuilderTask<Types>
where
    Types: NodeType,
{
    service: Arc<GlobalState<Types, Box<dyn BuilderHooks<Types>>>>,
}

impl<Types> BuilderTask<Types> for MarketplaceBuilderTask<Types>
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

    use hotshot_example_types::node_types::{MarketplaceTestVersions, MemoryImpl, TestTypes};
    use hotshot_macros::cross_tests;
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        overall_safety_task::OverallSafetyPropertiesDescription,
        test_builder::TestDescription,
    };
    use marketplace_builder_shared::testing::{
        generation::{self, TransactionGenerationConfig},
        run_test,
        validation::BuilderValidationConfig,
    };

    use crate::testing::integration::MarketplaceBuilderImpl;

    #[tokio::test(flavor = "multi_thread")]
    #[tracing::instrument]
    #[ignore = "slow"]
    async fn basic_test() {
        let num_successful_views = 45;
        let min_txns_per_view = 5;

        let mut metadata = TestDescription {
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

        run_test::<MarketplaceTestVersions, MarketplaceBuilderImpl>(
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
        .await
    }

    cross_tests!(
        TestName: example_cross_test,
        Impls: [MemoryImpl],
        BuilderImpls: [MarketplaceBuilderImpl],
        Types: [TestTypes],
        Versions: [MarketplaceTestVersions],
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
