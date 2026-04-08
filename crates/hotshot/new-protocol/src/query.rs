use std::sync::Arc;

use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    traits::node_implementation::NodeType,
    utils::StateAndDelta,
};
use oneshot::Sender;
use tokio::sync::oneshot;

pub enum CoordinatorQuery<T: NodeType> {
    CurrentView(Sender<ViewNumber>),
    CurrentEpoch(Sender<Option<EpochNumber>>),
    DecidedLeaf(Sender<Leaf2<T>>),
    DecidedState(Sender<Option<Arc<T::ValidatedState>>>),
    UndecidedLeaves(Sender<Vec<Leaf2<T>>>),
    GetState {
        view: ViewNumber,
        respond: Sender<Option<Arc<T::ValidatedState>>>,
    },
    GetStateAndDelta {
        view: ViewNumber,
        respond: Sender<StateAndDelta<T>>,
    },
}
