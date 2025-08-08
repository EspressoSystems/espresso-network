use chrono::{DateTime, Local};
use generation::TransactionGenerationConfig;
use hotshot_example_types::{
    block_types::TestTransaction,
    node_types::{MemoryImpl, TestTypes},
};
use hotshot_testing::{block_builder::TestBuilderImplementation, test_builder::TestDescription};
use hotshot_types::traits::node_implementation::Versions;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use validation::BuilderValidationConfig;

pub mod consensus;
pub mod constants;
pub mod generation;
pub mod mock;
pub mod validation;

/// Transaction payload used in testing to track various properties
/// of individual transactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionPayload {
    /// Monotonically increasing transaction number, to uniquely
    /// identify the transaction and track transaction ordering
    index: usize,
    /// Time of transaction creation
    created: DateTime<Local>,
    /// Arbitrary data to pad transaction length with
    data: Vec<u8>,
}

impl TransactionPayload {
    /// Create a new [`TransactionPayload`] with given `number`,
    /// `data_bytes` of randomly-generated padding and current
    /// time
    pub fn new(number: usize, data_bytes: usize) -> Self {
        let mut data = Vec::with_capacity(data_bytes);
        thread_rng().fill(data.as_mut_slice());
        Self {
            index: number,
            created: Local::now(),
            data,
        }
    }
}

impl TryFrom<&TransactionPayload> for TestTransaction {
    type Error = bincode::Error;

    fn try_from(value: &TransactionPayload) -> Result<Self, Self::Error> {
        Ok(TestTransaction::new(bincode::serialize(value)?))
    }
}

/// Run a builder test. Thin wrapper around `TestRunner::run_test`, injecting transaction generation
/// and validation tasks based on `validation_config` and `transaction_generation_config` respectively.
pub async fn run_test<V: Versions, BuilderImpl: TestBuilderImplementation<TestTypes>>(
    description: TestDescription<TestTypes, MemoryImpl, V>,
    validation_config: BuilderValidationConfig,
    transaction_generation_config: TransactionGenerationConfig,
) {
    let test_runner = hotshot_testing::test_builder::TestDescription::<
        TestTypes,
        MemoryImpl,
        V,
    >::gen_launcher_with_tasks(
        description,
        vec![
            Box::new(validation_config),
            Box::new(transaction_generation_config),
        ],
    )
    .launch();

    test_runner.run_test::<BuilderImpl>().await;
}
