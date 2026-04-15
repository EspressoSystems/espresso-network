use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use committable::{Commitment, Committable};
use hotshot::traits::{BlockPayload, ValidatedState};
use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2, VidCommitment, ViewNumber},
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        node_implementation::NodeType,
    },
    utils::BuilderCommitment,
    vote::HasViewNumber,
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, warn};

use crate::{
    helpers::{proposal_commitment, upgrade_lock},
    message::Proposal,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: BlockNumber,
    pub proposal: Proposal<T>,
    pub parent_commitment: Commitment<Leaf2<T>>,
    pub payload_size: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub builder_fee: BuilderFee<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateResponse<T: NodeType> {
    pub view: ViewNumber,
    pub commitment: Commitment<Leaf2<T>>,
    pub state: Arc<T::ValidatedState>,
    pub delta: Option<Delta<T>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeaderResponse<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum StateManagerOutput<T: NodeType> {
    State {
        response: StateResponse<T>,
        validated: bool,
    },
    Header {
        response: HeaderResponse<T>,
        header: Option<T::BlockHeader>,
    },
}

type Delta<T> = Arc<<<T as NodeType>::ValidatedState as ValidatedState<T>>::Delta>;

#[derive(Clone)]
pub struct StateEntry<T: NodeType> {
    pub state: Arc<T::ValidatedState>,
    pub delta: Option<Delta<T>>,
    pub leaf: Leaf2<T>,
}

pub struct StateManager<T: NodeType> {
    instance: Arc<T::InstanceState>,
    validated_states: BTreeMap<ViewNumber, StateEntry<T>>,
    state_requests: HashMap<Commitment<Leaf2<T>>, (AbortHandle, ViewNumber)>,
    header_requests: HashMap<ViewNumber, AbortHandle>,
    pending_requests: HashMap<Commitment<Leaf2<T>>, Vec<Pending<T>>>,
    tasks: JoinSet<Completed<T>>,
}

enum Pending<T: NodeType> {
    State(StateRequest<T>),
    Header(HeaderRequest<T>),
}

enum Completed<T: NodeType> {
    State {
        response: StateResponse<T>,
        leaf: Option<Leaf2<T>>,
    },
    Header {
        response: HeaderResponse<T>,
        header: Option<T::BlockHeader>,
    },
}

impl<T: NodeType> StateManager<T> {
    pub fn new(instance: Arc<T::InstanceState>) -> Self {
        Self {
            instance,
            validated_states: BTreeMap::new(),
            state_requests: HashMap::new(),
            header_requests: HashMap::new(),
            pending_requests: HashMap::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Get the validated state for a given view
    pub fn get_state(&self, view: &ViewNumber) -> Option<Arc<T::ValidatedState>> {
        self.validated_states
            .get(view)
            .map(|entry| entry.state.clone())
    }

    /// Get the validated state and delta for a given view
    pub fn get_state_and_delta(
        &self,
        view: &ViewNumber,
    ) -> (Option<Arc<T::ValidatedState>>, Option<Delta<T>>) {
        match self.validated_states.get(view) {
            Some(entry) => (Some(entry.state.clone()), entry.delta.clone()),
            None => (None, None),
        }
    }

    pub fn seed_state(&mut self, view: ViewNumber, state: Arc<T::ValidatedState>, leaf: Leaf2<T>) {
        self.insert_state(view, state, None, leaf);
    }

    pub fn request_state(&mut self, request: StateRequest<T>) {
        let commitment = proposal_commitment(&request.proposal);
        if self.state_requests.contains_key(&commitment) {
            return;
        }

        if self.state_requests.contains_key(&request.parent_commitment) {
            self.pending_requests
                .entry(request.parent_commitment)
                .or_default()
                .push(Pending::State(request));
            return;
        }

        let Some(parent_entry) = self.validated_states.get(&request.parent_view).cloned() else {
            self.insert_empty_state(request.proposal);
            self.start_pending(commitment);
            return;
        };

        let instance = self.instance.clone();
        let header = request.proposal.block_header.clone();
        let view = request.view;
        let payload_size = request.payload_size;

        let Ok(upgrade_lock) = upgrade_lock::<T>().version(view) else {
            error!(%view, "unsupported version");
            return;
        };

        let handle = self.tasks.spawn(async move {
            let result = parent_entry
                .state
                .validate_and_apply_header(
                    &instance,
                    &parent_entry.leaf,
                    &header,
                    payload_size,
                    upgrade_lock,
                    *view,
                )
                .await
                .map(|(state, delta)| StateResponse {
                    view,
                    commitment,
                    state: Arc::new(state),
                    delta: Some(Arc::new(delta)),
                });
            match result {
                Ok(response) => Completed::State {
                    response,
                    leaf: Some(request.proposal.into()),
                },
                Err(err) => {
                    warn!(%err, "state validation failed");
                    Completed::State {
                        response: StateResponse {
                            view,
                            commitment,
                            state: Arc::new(T::ValidatedState::from_header(&header)),
                            delta: None,
                        },
                        leaf: None,
                    }
                },
            }
        });

        self.state_requests.insert(commitment, (handle, view));
    }

    pub fn request_header(&mut self, request: HeaderRequest<T>) {
        if self.header_requests.contains_key(&request.view) {
            return;
        }

        let parent_commitment = proposal_commitment(&request.parent_proposal);

        if self.state_requests.contains_key(&parent_commitment) {
            self.pending_requests
                .entry(parent_commitment)
                .or_default()
                .push(Pending::Header(request));
            return;
        }

        let parent_view = request.parent_proposal.view_number();
        let Some(parent_entry) = self.validated_states.get(&parent_view).cloned() else {
            // Parent state not available yet (e.g. its proposal is still
            // being validated).  Queue the request so it is retried once
            // the state for the parent view is inserted.
            self.pending_requests
                .entry(parent_commitment)
                .or_default()
                .push(Pending::Header(request));
            return;
        };

        let instance = self.instance.clone();
        let view = request.view;
        let epoch = request.epoch;
        let parent_proposal = request.parent_proposal;

        let Ok(version) = upgrade_lock::<T>().version(view) else {
            error!(%view, "unsupported version");
            return;
        };

        let handle = self.tasks.spawn(async move {
            let result = T::BlockHeader::new(
                &parent_entry.state,
                &instance,
                &parent_entry.leaf,
                request.payload_commitment,
                request.builder_commitment,
                request.metadata,
                request.builder_fee,
                version,
                *view,
            )
            .await;
            match result {
                Ok(header) => Completed::Header {
                    response: HeaderResponse {
                        view,
                        epoch,
                        parent_proposal,
                    },
                    header: Some(header),
                },
                Err(err) => {
                    warn!(%err, "header creation failed");
                    Completed::Header {
                        response: HeaderResponse {
                            view,
                            epoch,
                            parent_proposal,
                        },
                        header: None,
                    }
                },
            }
        });

        self.header_requests.insert(view, handle);
    }

    /// Provide an externally-obtained validated state.
    pub fn update_state(&mut self, state: T::ValidatedState, view: ViewNumber, leaf: Leaf2<T>) {
        let commitment = leaf.commit();
        self.insert_state(view, Arc::new(state), None, leaf);
        if let Some((task, _)) = self.state_requests.remove(&commitment) {
            task.abort();
        }
        self.start_pending(commitment);
    }

    /// Get the next output.
    pub async fn next(&mut self) -> Option<StateManagerOutput<T>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => match result {
                    Completed::State {
                        response,
                        leaf: leaf2,
                    } => {
                        if self.state_requests.remove(&response.commitment).is_none() {
                            continue;
                        }
                        if let Some(leaf) = leaf2 {
                            self.insert_state(
                                response.view,
                                response.state.clone(),
                                response.delta.clone(),
                                leaf,
                            );
                            self.start_pending(response.commitment);
                            return Some(StateManagerOutput::State {
                                response,
                                validated: true,
                            });
                        } else {
                            self.pending_requests.remove(&response.commitment);
                            return Some(StateManagerOutput::State {
                                response,
                                validated: false,
                            });
                        }
                    },
                    Completed::Header { response, header } => {
                        if self.header_requests.remove(&response.view).is_none() {
                            continue;
                        }
                        return Some(StateManagerOutput::Header { response, header });
                    },
                },
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "task panicked");
                    }
                },
                None => return None,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        self.validated_states = self.validated_states.split_off(&view_number);
        for (task, view) in self.state_requests.values() {
            if *view < view_number {
                task.abort();
            }
        }
        self.state_requests
            .retain(|_, (_, view)| *view >= view_number);
    }

    fn start_pending(&mut self, finished_commitment: Commitment<Leaf2<T>>) {
        let Some(pending) = self.pending_requests.remove(&finished_commitment) else {
            return;
        };
        for p in pending {
            match p {
                Pending::State(r) => self.request_state(r),
                Pending::Header(r) => self.request_header(r),
            }
        }
    }

    /// Insert a state into the validated states map.
    ///
    /// States created via `from_header`
    /// have no delta. States produced by `validate_and_apply_header` carry a delta representing
    /// the state transition. This method prevents a `from_header` state from overwriting a
    /// fully validated state that already has a delta.
    fn insert_state(
        &mut self,
        view: ViewNumber,
        state: Arc<T::ValidatedState>,
        delta: Option<Delta<T>>,
        leaf: Leaf2<T>,
    ) {
        if let Some(existing) = self.validated_states.get(&view)
            && existing.delta.is_some()
            && delta.is_none()
        {
            warn!(
                ?view,
                "Skipping state update to not override a state with a delta"
            );
            return;
        }
        self.validated_states
            .insert(view, StateEntry { state, delta, leaf });
    }

    fn insert_empty_state(&mut self, proposal: Proposal<T>) {
        let state = T::ValidatedState::from_header(&proposal.block_header);
        self.insert_state(
            proposal.view_number(),
            Arc::new(state),
            None,
            proposal.into(),
        );
    }

    #[cfg(test)]
    pub(crate) fn validated_contains_view(&self, v: ViewNumber) -> bool {
        self.validated_states.contains_key(&v)
    }

    #[cfg(test)]
    pub(crate) fn pending_contains_commitment(&self, c: &Commitment<Leaf2<T>>) -> bool {
        self.pending_requests.contains_key(c)
    }
}
