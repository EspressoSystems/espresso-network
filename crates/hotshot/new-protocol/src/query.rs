use std::sync::Arc;

use committable::Commitment;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    message::Proposal as SignedProposal,
    traits::{ValidatedState, node_implementation::NodeType},
    utils::StateAndDelta,
};
use oneshot::Sender;
use tokio::sync::oneshot;

use crate::message::Proposal;

#[allow(clippy::large_enum_variant)]
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
    UpdateLeaf {
        view: ViewNumber,
        leaf: Leaf2<T>,
        state: Arc<T::ValidatedState>,
        delta: Option<Arc<<T::ValidatedState as ValidatedState<T>>::Delta>>,
        respond: Sender<anyhow::Result<()>>,
    },
    RequestProposal {
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
        respond: Sender<Option<SignedProposal<T, Proposal<T>>>>,
    },
}
