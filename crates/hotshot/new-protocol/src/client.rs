use std::{num::NonZeroUsize, sync::Arc};

use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    traits::node_implementation::NodeType,
    utils::StateAndDelta,
};
use tokio::sync::{mpsc, oneshot};

use crate::state::UpdateLeaf;

#[derive(Clone)]
pub struct ClientApi<T: NodeType> {
    tx: mpsc::Sender<ClientRequest<T>>,
}

impl<T: NodeType> ClientApi<T> {
    pub async fn current_view(&self) -> Result<ViewNumber, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::CurrentView(tx), rx).await
    }

    pub async fn current_epoch(&self) -> Result<Option<EpochNumber>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::CurrentEpoch(tx), rx).await
    }

    pub async fn decided_leaf(&self) -> Result<Leaf2<T>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::DecidedLeaf(tx), rx).await
    }

    pub async fn decided_state(&self) -> Result<Option<Arc<T::ValidatedState>>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::DecidedState(tx), rx).await
    }

    pub async fn undecided_leaves(&self) -> Result<Vec<Leaf2<T>>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::UndecidedLeaves(tx), rx).await
    }

    pub async fn state(
        &self,
        view: ViewNumber,
    ) -> Result<Option<Arc<T::ValidatedState>>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::GetState { view, respond: tx }, rx)
            .await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> Result<StateAndDelta<T>, QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(ClientRequest::GetStateAndDelta { view, respond: tx }, rx)
            .await
    }

    pub async fn update_leaf(&self, update: UpdateLeaf<T>) -> Result<(), QueryError> {
        let (tx, rx) = oneshot::channel();
        self.call(
            ClientRequest::UpdateLeaf {
                update,
                respond: tx,
            },
            rx,
        )
        .await
    }

    async fn call<A>(
        &self,
        request: ClientRequest<T>,
        rx: oneshot::Receiver<A>,
    ) -> Result<A, QueryError> {
        self.tx
            .send(request)
            .await
            .map_err(|_| QueryError::ChannelClosed)?;
        rx.await.map_err(|_| QueryError::ResponseDropped)
    }
}

/// The coordinator client owns the receive end of the request channel.
///
/// The coordinator holds this and calls [`next_request`](CoordinatorClient::next_request)
/// to process incoming requests.
pub struct CoordinatorClient<T: NodeType> {
    rx: mpsc::Receiver<ClientRequest<T>>,
    api: ClientApi<T>,
}

impl<T: NodeType> Default for CoordinatorClient<T> {
    fn default() -> Self {
        Self::new(NonZeroUsize::new(256).expect("256 > 0"))
    }
}

impl<T: NodeType> CoordinatorClient<T> {
    pub fn new(capacity: NonZeroUsize) -> Self {
        let (tx, rx) = mpsc::channel(capacity.get());
        Self {
            rx,
            api: ClientApi { tx },
        }
    }

    pub fn handle(&self) -> &ClientApi<T> {
        &self.api
    }

    pub(crate) async fn next_request(&mut self) -> Option<ClientRequest<T>> {
        self.rx.recv().await
    }
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum ClientRequest<T: NodeType> {
    CurrentView(oneshot::Sender<ViewNumber>),
    CurrentEpoch(oneshot::Sender<Option<EpochNumber>>),
    DecidedLeaf(oneshot::Sender<Leaf2<T>>),
    DecidedState(oneshot::Sender<Option<Arc<T::ValidatedState>>>),
    UndecidedLeaves(oneshot::Sender<Vec<Leaf2<T>>>),
    GetState {
        view: ViewNumber,
        respond: oneshot::Sender<Option<Arc<T::ValidatedState>>>,
    },
    GetStateAndDelta {
        view: ViewNumber,
        respond: oneshot::Sender<StateAndDelta<T>>,
    },
    UpdateLeaf {
        update: UpdateLeaf<T>,
        respond: oneshot::Sender<()>,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    #[error("failed to send request. coordinator channel closed")]
    ChannelClosed,

    #[error("coordinator dropped the response")]
    ResponseDropped,
}
