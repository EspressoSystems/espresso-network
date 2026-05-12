use std::{collections::BTreeMap, num::NonZeroUsize, sync::Arc};

use async_trait::async_trait;
use committable::Commitment;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    message::Proposal as SignedProposal,
    simple_certificate::QuorumCertificate2,
    simple_vote::TimeoutVote2,
    traits::{leaf_fetcher_network::LeafFetcherNetwork, node_implementation::NodeType},
    utils::StateAndDelta,
};
use tokio::sync::{mpsc, oneshot};

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
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(
            ClientRequest::SendExternalMessage {
                view,
                payload,
                recipient,
                respond,
            },
            rx,
        )
        .await?
    }

    /// Forward a `TimeoutVote2` produced by the legacy (pre-0.8) consensus
    /// task into the new-protocol coordinator's timeout collectors. Used at
    /// the legacy → new-protocol boundary: when a legacy view near the
    /// cutover times out, the legacy task signs a `TimeoutVote2` (whose
    /// commitment is version-tagged via the shared `UpgradeLock`) and
    /// submits it here so the first 0.8 leader can collect a
    /// `TimeoutCertificate2` for that pre-cutover view.
    ///
    /// `TimeoutVote2` is structurally identical between 0.4 and 0.8
    /// (`SimpleVote<TYPES, TimeoutData2>`) so the same vote feeds both
    /// systems' aggregators without re-signing.
    pub async fn submit_timeout_vote(&self, vote: TimeoutVote2<T>) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(ClientRequest::SubmitTimeoutVote { vote, respond }, rx)
            .await
    }

    /// Refresh the coordinator network's peer set for `epoch`.
    ///
    /// Used during the legacy → new-protocol phase to keep the coordinator's
    /// `Cliquenet` connected to the live stake-table window even though no
    /// new-protocol proposals are flowing yet (the only other call site for
    /// `Network::on_epoch_change` is on a validated proposal). Without this,
    /// a node that stayed up across many legacy epoch transitions would
    /// arrive at the cutover with peers from the boot epoch's window.
    ///
    /// Idempotent at the network layer: `Cliquenet::on_epoch_change`
    /// short-circuits when `epoch <= self.epoch`.
    pub async fn bump_network_epoch(&self, epoch: EpochNumber) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(ClientRequest::BumpNetworkEpoch { epoch, respond }, rx)
            .await
    }

    /// Bridge legacy (pre-0.8) state into the running coordinator at the
    /// legacy → new-protocol cutover.
    ///
    /// - `decided_anchor` is the highest leaf 0.4 had decided.
    /// - `undecided` is the chain of undecided 0.4 leaves above the anchor
    ///   (oldest-first).
    /// - `high_qc` is the QC of the topmost undecided leaf, if 0.4 voting
    ///   completed enough for that QC to form. Required for the first 0.8
    ///   leader to find `certs[N-1]` when proposing at view N (= the
    ///   topmost leaf's view + 1). May be `None` if the chain stalled
    ///   before the topmost leaf got a QC; in that case the first 0.8
    ///   leader will need view-change evidence.
    /// - `validated_states` is the validated state of every seeded leaf
    ///   (anchor + undecided), keyed by view number. The new protocol
    ///   pipelines header creation and state validation against the
    ///   parent's stored state — without seeding these, the first
    ///   post-cutover leader cannot build a header (no parent state) and
    ///   peers cannot validate the first post-cutover proposal.
    /// - `cutover_view` is the upgrade certificate's
    ///   `new_version_first_view`. The new protocol must never propose,
    ///   vote on, or decide any view strictly below this — those views
    ///   belong to legacy, even when legacy left some without QCs.
    ///
    /// Idempotent at the consensus level: `set_pre_cutover_anchor` no-ops if
    /// the supplied view is not above the current `last_decided_view`, and
    /// `seed_pre_cutover_leaves` reinserts views that are already in the set.
    pub async fn seed_pre_cutover(
        &self,
        decided_anchor: Leaf2<T>,
        undecided: Vec<Leaf2<T>>,
        high_qc: Option<QuorumCertificate2<T>>,
        validated_states: BTreeMap<ViewNumber, Arc<T::ValidatedState>>,
        cutover_view: ViewNumber,
    ) -> Result<(), QueryError> {
        let (respond, rx) = oneshot::channel();
        self.call(
            ClientRequest::SeedPreCutover {
                decided_anchor,
                undecided,
                high_qc,
                validated_states,
                cutover_view,
                respond,
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
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
        respond: oneshot::Sender<Result<(), QueryError>>,
    },
    SeedPreCutover {
        decided_anchor: Leaf2<T>,
        undecided: Vec<Leaf2<T>>,
        cutover_view: ViewNumber,
        high_qc: Option<QuorumCertificate2<T>>,
        /// Validated state for each seeded leaf, keyed by view. Empty if
        /// the caller has no states to seed (e.g. legacy-only test paths).
        validated_states: BTreeMap<ViewNumber, Arc<T::ValidatedState>>,
        respond: oneshot::Sender<()>,
    },
    SubmitTimeoutVote {
        vote: TimeoutVote2<T>,
        respond: oneshot::Sender<()>,
    },
    BumpNetworkEpoch {
        epoch: EpochNumber,
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
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> anyhow::Result<()> {
        self.client
            .send_external_message(view, payload, recipient)
            .await?;
        Ok(())
    }

    async fn send_leaf_response(
        &self,
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> anyhow::Result<()> {
        self.client
            .send_external_message(view, payload, recipient)
            .await?;
        Ok(())
    }
}
