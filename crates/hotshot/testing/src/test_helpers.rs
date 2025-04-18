// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use committable::Committable;
use hotshot_example_types::{node_types::TestTypes, state_types::TestValidatedState};
use hotshot_types::{
    data::Leaf,
    utils::{View, ViewInner},
};
/// This function will create a fake [`View`] from a provided [`Leaf`].
pub fn create_fake_view_with_leaf(leaf: Leaf<TestTypes>) -> View<TestTypes> {
    create_fake_view_with_leaf_and_state(leaf, TestValidatedState::default())
}

/// This function will create a fake [`View`] from a provided [`Leaf`] and `state`.
pub fn create_fake_view_with_leaf_and_state(
    leaf: Leaf<TestTypes>,
    state: TestValidatedState,
) -> View<TestTypes> {
    View {
        view_inner: ViewInner::Leaf {
            leaf: leaf.commit(),
            state: state.into(),
            delta: None,
        },
    }
}
