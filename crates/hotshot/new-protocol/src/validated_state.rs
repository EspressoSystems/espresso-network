use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use committable::{Commitment, Committable};
use hotshot::traits::ValidatedState;
use hotshot_types::{
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{HeaderRequest, StateEvent, StateRequest, StateResponse},
    helpers::{proposal_commitment, upgrade_lock},
};

type StateError<TYPES> = <<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Error;
type HeaderError<TYPES> = <<TYPES as NodeType>::BlockHeader as BlockHeader<TYPES>>::Error;

type InProgressRequest<TYPES> = (JoinHandle<()>, StateRequest<TYPES>);

enum CompletedRequest<TYPES: NodeType> {
    State(Result<StateResponse<TYPES>, StateError<TYPES>>),
    Header(Result<(ViewNumber, TYPES::BlockHeader), HeaderError<TYPES>>),
}

struct ValidatedStateManager<TYPES: NodeType> {
    validated_states: BTreeMap<ViewNumber, (Arc<TYPES::ValidatedState>, Leaf2<TYPES>)>,
    in_progress_requests: HashMap<Commitment<Leaf2<TYPES>>, InProgressRequest<TYPES>>,
    in_progress_headers: HashMap<ViewNumber, JoinHandle<()>>,
    pending_requests: BTreeMap<Commitment<Leaf2<TYPES>>, Vec<StateEvent<TYPES>>>,

    event_rx: Receiver<StateEvent<TYPES>>,
    completed_requests_tx: Sender<CompletedRequest<TYPES>>,
    completed_requests_rx: Receiver<CompletedRequest<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,

    instance_state: Arc<TYPES::InstanceState>,
}

impl<TYPES: NodeType> ValidatedStateManager<TYPES> {
    pub fn new(
        event_rx: Receiver<StateEvent<TYPES>>,
        instance_state: Arc<TYPES::InstanceState>,
        coordinator_handle: CoordinatorHandle<TYPES>,
    ) -> Self {
        let (completed_requests_tx, completed_requests_rx) = mpsc::channel(100);
        Self {
            validated_states: BTreeMap::new(),
            in_progress_requests: HashMap::new(),
            in_progress_headers: HashMap::new(),
            pending_requests: BTreeMap::new(),
            event_rx,
            completed_requests_tx,
            completed_requests_rx,
            coordinator_handle,
            instance_state,
        }
    }

    async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await;
                },
                Some(completed_request) = self.completed_requests_rx.recv() => {
                    self.handle_completed_request(completed_request).await;
                },
            }
        }
    }

    async fn handle_completed_request(
        &mut self,
        completed_request: CompletedRequest<TYPES>,
    ) -> Option<()> {
        match completed_request {
            CompletedRequest::State(result) => self.handle_state_completed(result).await?,
            CompletedRequest::Header(result) => self.handle_header_completed(result).await?,
        }
        Some(())
    }

    async fn handle_event(&mut self, event: StateEvent<TYPES>) {
        match event {
            StateEvent::RequestState(request) => self.handle_request_state(request).await,
            StateEvent::RequestHeader(request) => self.handle_request_header(request).await,
            StateEvent::UpdateState(state, view, leaf) => {
                self.handle_update_state(state, view, leaf).await
            },
        }
    }

    async fn handle_state_completed(
        &mut self,
        state: Result<StateResponse<TYPES>, StateError<TYPES>>,
    ) -> Option<()> {
        match state {
            Ok(response) => {
                let (_, request) = self.in_progress_requests.remove(&response.commitment)?;
                self.coordinator_handle
                    .respond_state(request.clone())
                    .await
                    .ok()?;
                let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper::<TYPES> {
                    proposal: request.proposal,
                });
                self.validated_states
                    .insert(response.view, (response.state, leaf));
                self.start_pending(response.commitment).await;
            },
            Err(e) => {
                self.handle_state_error(e).await;
            },
        }
        Some(())
    }

    async fn handle_state_error(&mut self, error: StateError<TYPES>) {
        // TODO: We need to wrap the error with more information
        // so we can send back
        tracing::error!("Failed to handle state completed: {}", error);
    }

    async fn handle_header_completed(
        &mut self,
        result: Result<(ViewNumber, TYPES::BlockHeader), HeaderError<TYPES>>,
    ) -> Option<()> {
        match result {
            Ok((view, header)) => {
                self.in_progress_headers.remove(&view);
                self.coordinator_handle
                    .respond_header(view, header)
                    .await
                    .ok()?;
            },
            Err(error) => {
                self.handle_header_error(error).await;
            },
        }
        Some(())
    }
    async fn handle_header_error(&mut self, error: HeaderError<TYPES>) {
        // TODO: We need to wrap the error with more information
        // so we can send back
        tracing::error!("Failed to handle header completed: {}", error);
    }

    fn is_in_progress(&self, commitment: Commitment<Leaf2<TYPES>>) -> bool {
        self.in_progress_requests.contains_key(&commitment)
    }

    fn insert_pending_state(
        &mut self,
        commitment: Commitment<Leaf2<TYPES>>,
        request: StateRequest<TYPES>,
    ) {
        self.pending_requests
            .entry(commitment)
            .or_default()
            .push(StateEvent::RequestState(request));
    }
    fn insert_pending_header(
        &mut self,
        commitment: Commitment<Leaf2<TYPES>>,
        request: HeaderRequest<TYPES>,
    ) {
        self.pending_requests
            .entry(commitment)
            .or_default()
            .push(StateEvent::RequestHeader(request));
    }

    fn insert_empty_state(&mut self, proposal: QuorumProposal2<TYPES>) {
        let state = TYPES::ValidatedState::from_header(&proposal.block_header);
        self.validated_states.insert(
            proposal.view_number(),
            (
                Arc::new(state),
                Leaf2::from_quorum_proposal(&QuorumProposalWrapper::<TYPES> { proposal }),
            ),
        );
    }

    #[allow(clippy::too_many_arguments)]
    async fn spawn_validate(
        &self,
        parent_state: Arc<TYPES::ValidatedState>,
        instance_state: Arc<TYPES::InstanceState>,
        parent_leaf: Leaf2<TYPES>,
        header: TYPES::BlockHeader,
        payload_size: u32,
        view_number: u64,
        commitment: Commitment<Leaf2<TYPES>>,
        tx: Sender<CompletedRequest<TYPES>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let state_result = parent_state
                .validate_and_apply_header(
                    instance_state.as_ref(),
                    &parent_leaf,
                    &header,
                    payload_size,
                    upgrade_lock::<TYPES>().version(view_number.into()).unwrap(),
                    view_number,
                )
                .await;
            let response = state_result.map(|(state, _delta)| StateResponse {
                view: view_number.into(),
                commitment,
                state: Arc::new(state),
            });
            tx.send(CompletedRequest::State(response)).await.unwrap();
        })
    }

    async fn handle_request_state(&mut self, request: StateRequest<TYPES>) {
        let commitment = proposal_commitment(&request.proposal);
        if self.is_in_progress(commitment) {
            return;
        }
        // Wait for the parent state to be completed, then calculate to avoid double catchup
        if self.is_in_progress(request.parent_commitment) {
            self.insert_pending_state(request.parent_commitment, request);
            return;
        }

        // if we don't have the parent state, we can't apply this header
        // add an empty state so we can apply the next header
        let Some((parent_state, parent_leaf)) =
            self.validated_states.get(&request.parent_view).cloned()
        else {
            self.insert_empty_state(request.proposal);
            return;
        };

        let completed_requests_tx = self.completed_requests_tx.clone();
        let instance_state = self.instance_state.clone();

        let handle = self
            .spawn_validate(
                parent_state,
                instance_state,
                parent_leaf,
                request.proposal.block_header.clone(),
                request.payload_size,
                *request.view,
                commitment,
                completed_requests_tx,
            )
            .await;
        self.in_progress_requests
            .insert(commitment, (handle, request));
    }

    async fn handle_request_header(&mut self, request: HeaderRequest<TYPES>) {
        if self.is_header_in_progress(request.view) {
            return;
        }
        let parent_commitment = proposal_commitment(&request.parent_proposal);
        if self.is_in_progress(parent_commitment) {
            self.insert_pending_header(parent_commitment, request);
            return;
        }

        let parent_view = request.parent_proposal.view_number();

        let Some((parent_state, parent_leaf)) = self.validated_states.get(&parent_view).cloned()
        else {
            tracing::error!(
                "Parent state not found for header request: {}",
                request.view
            );
            return;
        };

        let instance_state = self.instance_state.clone();

        let completed_requests_tx = self.completed_requests_tx.clone();

        let handle = tokio::spawn(async move {
            let header = TYPES::BlockHeader::new(
                parent_state.as_ref(),
                instance_state.as_ref(),
                &parent_leaf,
                request.payload_commitment,
                request.builder_commitment,
                request.metadata,
                request.builder_fee,
                upgrade_lock::<TYPES>().version(request.view).unwrap(),
                *request.view,
            )
            .await;
            let response = header.map(|header| (request.view, header));
            completed_requests_tx
                .send(CompletedRequest::Header(response))
                .await
                .unwrap();
        });
    }

    fn is_header_in_progress(&self, view: ViewNumber) -> bool {
        self.in_progress_headers.contains_key(&view)
    }

    async fn start_pending(&mut self, finished_commitment: Commitment<Leaf2<TYPES>>) {
        let Some(pending_requests) = self.pending_requests.remove(&finished_commitment) else {
            return;
        };
        for event in pending_requests {
            self.handle_event(event).await;
        }
    }

    async fn handle_update_state(
        &mut self,
        state: TYPES::ValidatedState,
        view: ViewNumber,
        leaf: Leaf2<TYPES>,
    ) {
        let commitment = leaf.commit();
        self.validated_states.insert(view, (Arc::new(state), leaf));
        self.in_progress_requests.remove(&commitment);
        self.start_pending(commitment).await;
    }
}
