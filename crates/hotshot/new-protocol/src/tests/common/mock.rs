use consensus_store::mem::MemStore;
use hotshot_example_types::node_types::TestTypes;

use crate::coordinator::Coordinator;

pub type MockCoordinator = Coordinator<TestTypes, MemStore>;
