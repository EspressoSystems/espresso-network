use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

use async_trait::async_trait;
use committable::Commitment;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    message::Proposal as SignedProposal,
    simple_certificate::QuorumCertificate2,
    simple_vote::TimeoutVote2,
    traits::{
        leaf_fetcher_network::LeafFetcherNetwork, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    utils::StateAndDelta,
};
use tokio::sync::{mpsc, mpsc::error::TrySendError, oneshot};

use crate::{coordinator::error::CoordinatorError, message::Proposal, state::UpdateLeaf};

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

    pub async fn submit_transaction(&self, tx: T::Transaction) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(ClientRequest::SubmitTransaction { tx, respond }, rx)
            .await
    }

    pub async fn request_proposal(
        &self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> Result<SignedProposal<T, Proposal<T>>, QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(
            ClientRequest::RequestProposal {
                view,
                leaf_commitment,
                respond,
            },
            rx,
        )
        .await?
    }

    pub async fn send_external_message(
        &self,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(
            ClientRequest::SendExternalMessage {
                payload,
                recipient,
                respond,
            },
            rx,
        )
        .await?
    }

    /// Proposal participation ratios; `None` queries the current epoch.
    pub async fn proposal_participation(
        &self,
        epoch: Option<EpochNumber>,
    ) -> Result<HashMap<T::SignatureKey, f64>, QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(ClientRequest::ProposalParticipation { epoch, respond }, rx)
            .await
    }

    /// Vote participation ratios; `None` queries the current epoch.
    pub async fn vote_participation(
        &self,
        epoch: Option<EpochNumber>,
    ) -> Result<HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64>, QueryError>
    {
        let (respond, rx) = oneshot::channel();
        self.call(ClientRequest::VoteParticipation { epoch, respond }, rx)
            .await
    }

    /// Fire-and-forget a legacy `TimeoutVote2` into the new-protocol timeout
    /// collectors.
    pub fn submit_timeout_vote(&self, vote: TimeoutVote2<T>) -> Result<(), QueryError> {
        self.try_send(ClientRequest::SubmitTimeoutVote { vote })
    }

    /// Fire-and-forget the last legacy view's QC so the first new-protocol
    /// leader can propose on it even if the cutover seed was snapshotted
    /// before it formed.
    pub fn submit_legacy_high_qc(&self, qc: QuorumCertificate2<T>) -> Result<(), QueryError> {
        self.try_send(ClientRequest::SubmitLegacyHighQc { qc })
    }

    /// Fire-and-forget a refresh of the coordinator network's peer set for
    /// `epoch`.
    pub fn bump_network_epoch(&self, epoch: EpochNumber) -> Result<(), QueryError> {
        self.try_send(ClientRequest::BumpNetworkEpoch { epoch })
    }

    /// Send `request` without waiting for a response. The fire-and-forget
    /// bridge methods above use this because they must never block on a
    /// coordinator that hasn't started processing requests yet: requests
    /// queue in the bounded request channel until the coordinator starts and
    /// are dropped with an error when the queue is full.
    fn try_send(&self, request: ClientRequest<T>) -> Result<(), QueryError> {
        self.tx.try_send(request).map_err(|err| match err {
            TrySendError::Closed(_) => QueryError::ChannelClosed,
            TrySendError::Full(_) => QueryError::ChannelFull,
        })
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
    ProposalParticipation {
        epoch: Option<EpochNumber>,
        respond: oneshot::Sender<HashMap<T::SignatureKey, f64>>,
    },
    VoteParticipation {
        epoch: Option<EpochNumber>,
        respond:
            oneshot::Sender<HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64>>,
    },
    UpdateLeaf {
        update: UpdateLeaf<T>,
        respond: oneshot::Sender<()>,
    },
    SubmitTransaction {
        tx: T::Transaction,
        respond: oneshot::Sender<()>,
    },
    RequestProposal {
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
        respond: oneshot::Sender<Result<SignedProposal<T, Proposal<T>>, QueryError>>,
    },
    SendExternalMessage {
        payload: Vec<u8>,
        recipient: T::SignatureKey,
        respond: oneshot::Sender<Result<(), QueryError>>,
    },
    SubmitTimeoutVote {
        vote: TimeoutVote2<T>,
    },
    SubmitLegacyHighQc {
        qc: QuorumCertificate2<T>,
    },
    BumpNetworkEpoch {
        epoch: EpochNumber,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    #[error("failed to send request. coordinator channel closed")]
    ChannelClosed,

    #[error("coordinator dropped the response")]
    ResponseDropped,

    #[error("request dropped: coordinator request queue is full")]
    ChannelFull,

    #[error("coordinator error: {0}")]
    Coordinator(#[from] CoordinatorError),
}

/// `LeafFetcherNetwork` impl that routes catchup direct-messages through
/// the `Coordinator`'s single owned network via [`ClientApi`].
///
/// The membership layer gets a clone of this so it does not need its own
/// network handle — the `Coordinator` is the only owner of the underlying
/// `ConnectedNetwork`.
pub struct ClientLeafFetcherNetwork<T: NodeType> {
    client: ClientApi<T>,
}

impl<T: NodeType> ClientLeafFetcherNetwork<T> {
    pub fn new(client: ClientApi<T>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<T: NodeType> LeafFetcherNetwork<T> for ClientLeafFetcherNetwork<T> {
    async fn send_leaf_request(
        &self,
        _: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> anyhow::Result<()> {
        self.client
            .send_external_message(payload, recipient)
            .await?;
        Ok(())
    }

    async fn send_leaf_response(
        &self,
        _: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> anyhow::Result<()> {
        self.client
            .send_external_message(payload, recipient)
            .await?;
        Ok(())
    }
}
