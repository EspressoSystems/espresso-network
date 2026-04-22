use hotshot_example_types::node_types::TestTypes;

use crate::{coordinator::Coordinator, network::cliquenet::Cliquenet};

pub type MockCoordinator = Coordinator<TestTypes, Cliquenet<TestTypes>>;
