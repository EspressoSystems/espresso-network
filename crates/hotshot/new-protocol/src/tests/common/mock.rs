use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};

use crate::{coordinator::Coordinator, network::cliquenet::Cliquenet};

pub type MockCoordinator = Coordinator<TestTypes, Cliquenet<TestTypes>, TestStorage<TestTypes>>;
