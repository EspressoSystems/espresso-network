// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{rc::Rc, sync::Arc};

use hotshot::traits::{NodeImplementation, TestableNodeImplementation};
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    HotShotConfig, ValidatorConfig,
    traits::{network::AsyncGenerator, node_implementation::NodeType},
};

use super::test_builder::TestDescription;

/// A type alias to help readability
pub type Network<TYPES, I> = Arc<<I as NodeImplementation<TYPES>>::Network>;

/// Wrapper for a function that takes a `node_id` and returns an instance of `T`.
pub type Generator<T> = Rc<dyn Fn(u64) -> T>;

/// generators for resources used by each node
pub struct ResourceGenerators<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> {
    /// generate channels
    pub channel_generator: AsyncGenerator<Network<TYPES, I>>,
    /// generate new storage for each node
    pub storage: Generator<TestStorage<TYPES>>,
    /// configuration used to generate each hotshot node
    pub hotshot_config: Generator<HotShotConfig<TYPES>>,
    /// config that contains the signature keys
    pub validator_config: Generator<ValidatorConfig<TYPES>>,
}

/// test launcher
pub struct TestLauncher<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> {
    /// generator for resources
    pub resource_generators: ResourceGenerators<TYPES, I>,
    /// metadata used for tasks
    pub metadata: TestDescription<TYPES>,
}

impl<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> TestLauncher<TYPES, I> {
    /// Modifies the config used when generating nodes with `f`
    #[must_use]
    pub fn map_hotshot_config(mut self, f: impl Fn(&mut HotShotConfig<TYPES>) + 'static) -> Self {
        let mut test_config = self.metadata.test_config.clone();
        f(&mut test_config);

        let hotshot_config_generator = self.resource_generators.hotshot_config.clone();
        let hotshot_config: Generator<_> = Rc::new(move |node_id| {
            let mut result = (hotshot_config_generator)(node_id);
            f(&mut result);

            result
        });

        self.metadata.test_config = test_config;
        self.resource_generators.hotshot_config = hotshot_config;

        self
    }
}
