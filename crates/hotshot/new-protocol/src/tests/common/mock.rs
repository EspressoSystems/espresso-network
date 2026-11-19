use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};

use crate::coordinator::Coordinator;

pub type MockCoordinator = Coordinator<TestTypes, TestStorage<TestTypes>>;
