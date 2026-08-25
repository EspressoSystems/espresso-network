use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, SystemTime},
};

use committable::{Commitment, Committable};
use hotshot::traits::{BlockPayload, ValidatedState};
use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2, VidCommitment, ViewNumber},
    message::UpgradeLock,
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        metrics::Histogram,
        node_implementation::NodeType,
    },
    utils::BuilderCommitment,
    vote::HasViewNumber,
};
use tokio::{
    task::{AbortHandle, JoinSet},
    time::sleep,
};
use tracing::{error, warn};

use crate::{
    coordinator::metrics::{Measurement, finish_measurement, ignore_measurement},
    helpers::proposal_commitment,
    message::Proposal,
};

const DEFAULT_PARENT_DEADLINE: Duration = Duration::from_secs(5);

pub struct UpdateLeaf<T: NodeType> {
    pub view: ViewNumber,
    pub leaf: Leaf2<T>,
    pub state: Arc<T::ValidatedState>,
    pub delta: Option<Delta<T>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: BlockNumber,
    pub proposal: Proposal<T>,
    pub parent_commitment: Commitment<Leaf2<T>>,
    pub payload_size: u32,
    pub received_at: SystemTime,
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
    validated_states: BTreeMap<Commitment<Leaf2<T>>, StateEntry<T>>,
    state_requests: HashMap<Commitment<Leaf2<T>>, InFlight<T>>,
    header_requests: HashMap<(ViewNumber, Commitment<Leaf2<T>>), AbortHandle>,
    pending_requests: HashMap<Commitment<Leaf2<T>>, Vec<Pending<T>>>,
    upgrade_lock: UpgradeLock<T>,
    tasks: JoinSet<Completed<T>>,
    validate_duration_metric: Option<Arc<dyn Histogram>>,
    update_leaf_duration_metric: Option<Arc<dyn Histogram>>,
    parent_deadline: Duration,
}

/// A state validation in progress. The proposal lets `gc` seed a stub for a
/// validation it aborts.
struct InFlight<T: NodeType> {
    validation: AbortHandle,
    deadline: AbortHandle,
    view: ViewNumber,
    proposal: Proposal<T>,
    /// Outlived the parent deadline; requests no longer wait for it.
    overdue: bool,
}

impl<T: NodeType> InFlight<T> {
    fn abort(&self) {
        self.validation.abort();
        self.deadline.abort();
    }
}

enum Pending<T: NodeType> {
    State(StateRequest<T>),
    Header(HeaderRequest<T>),
}

impl<T: NodeType> Pending<T> {
    fn view(&self) -> ViewNumber {
        match self {
            Pending::State(r) => r.view,
            Pending::Header(r) => r.view,
        }
    }
}

enum Completed<T: NodeType> {
    State {
        response: StateResponse<T>,
        leaf: Leaf2<T>,
        validated: bool,
    },
    Header {
        response: HeaderResponse<T>,
        header: Option<T::BlockHeader>,
    },
    Deadline(Proposal<T>),
}

impl<T: NodeType> StateManager<T> {
    pub fn new(instance: Arc<T::InstanceState>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            instance,
            validated_states: BTreeMap::new(),
            state_requests: HashMap::new(),
            header_requests: HashMap::new(),
            pending_requests: HashMap::new(),
            upgrade_lock,
            tasks: JoinSet::new(),
            validate_duration_metric: None,
            update_leaf_duration_metric: None,
            parent_deadline: DEFAULT_PARENT_DEADLINE,
        }
    }

    /// How long requests wait for a parent's in-flight validation before
    /// proceeding against its `from_header` stub. A validation this slow is
    /// catchup-bound, and queueing more catchups behind it costs more than
    /// running them in parallel.
    pub fn with_parent_deadline(mut self, deadline: Duration) -> Self {
        self.parent_deadline = deadline;
        self
    }

    pub fn with_metrics(
        mut self,
        validate: Option<Arc<dyn Histogram>>,
        update_leaf: Option<Arc<dyn Histogram>>,
    ) -> Self {
        self.validate_duration_metric = validate;
        self.update_leaf_duration_metric = update_leaf;
        self
    }

    /// Get the validated state for a given view.
    pub fn get_state(&self, view: ViewNumber) -> Option<&StateEntry<T>> {
        self.validated_states
            .iter()
            .find(|(_, entry)| entry.leaf.view_number() == view)
            .map(|(_, entry)| entry)
    }

    /// Get the leaf for a given view
    pub fn get_leaf(&self, view: ViewNumber) -> Option<Leaf2<T>> {
        self.validated_states
            .iter()
            .find(|(_, entry)| entry.leaf.view_number() == view)
            .map(|(_, entry)| entry.leaf.clone())
    }

    pub fn seed_state(&mut self, view: ViewNumber, state: Arc<T::ValidatedState>, leaf: Leaf2<T>) {
        self.insert_state(view, state, None, leaf);
    }

    /// Seed a commitment-only (`from_header`) state so a child proposal can be
    /// validated against this leaf via catchup instead of being dropped.
    ///
    /// A real (validated) state for the leaf is never displaced, and any
    /// header/state requests already queued on this leaf are restarted.
    pub(crate) fn seed_from_header(&mut self, proposal: Proposal<T>) {
        let commitment = proposal_commitment(&proposal);
        if !self.validated_states.contains_key(&commitment) {
            self.insert_empty_state(proposal);
        }
        self.start_pending(commitment);
    }

    pub fn request_state(&mut self, request: StateRequest<T>) {
        let commitment = proposal_commitment(&request.proposal);
        if self.state_requests.contains_key(&commitment) {
            return;
        }

        if self.parent_in_flight(&request.parent_commitment) {
            self.pending_requests
                .entry(request.parent_commitment)
                .or_default()
                .push(Pending::State(request));
            return;
        }

        let Some(parent_entry) = self
            .validated_states
            .get(&request.parent_commitment)
            .cloned()
        else {
            warn!(
                view = %request.view,
                parent_view = %request.parent_view,
                epoch = %request.epoch,
                block = %request.block,
                parent_commitment = %request.parent_commitment,
                "parent state unavailable; queued on parent for retry (from_header stub inserted). \
                 If this persists, the parent state never arrived and the node cannot vote."
            );
            self.insert_empty_state(request.proposal.clone());
            let queued = self
                .pending_requests
                .entry(request.parent_commitment)
                .or_default();
            if !queued
                .iter()
                .any(|p| matches!(p, Pending::State(r) if r.view == request.view))
            {
                queued.push(Pending::State(request));
            }
            self.start_pending(commitment);
            return;
        };

        let instance = self.instance.clone();
        let header = request.proposal.block_header.clone();
        let proposal = request.proposal.clone();
        let view = request.view;
        let payload_size = request.payload_size;

        let Ok(upgrade_lock) = self.upgrade_lock.version(view) else {
            error!(%view, "unsupported version");
            return;
        };

        let duration_metric = self.validate_duration_metric.clone();
        let validation = self.tasks.spawn(async move {
            let measurement = duration_metric.map(Measurement::start);
            let result = parent_entry
                .state
                .validate_and_apply_header(
                    &instance,
                    &parent_entry.leaf,
                    &header,
                    payload_size,
                    upgrade_lock,
                    *view,
                    request.received_at,
                )
                .await;
            let leaf = request.proposal.into();
            match result {
                Ok((state, delta)) => {
                    finish_measurement(measurement);
                    Completed::State {
                        response: StateResponse {
                            view,
                            commitment,
                            state: Arc::new(state),
                            delta: Some(Arc::new(delta)),
                        },
                        leaf,
                        validated: true,
                    }
                },
                Err(err) => {
                    ignore_measurement(measurement);
                    warn!(%err, "state validation failed");
                    Completed::State {
                        response: StateResponse {
                            view,
                            commitment,
                            state: Arc::new(T::ValidatedState::from_header(&header)),
                            delta: None,
                        },
                        leaf,
                        validated: false,
                    }
                },
            }
        });
        let parent_deadline = self.parent_deadline;
        let deadline_proposal = proposal.clone();
        let deadline = self.tasks.spawn(async move {
            sleep(parent_deadline).await;
            Completed::Deadline(deadline_proposal)
        });

        self.state_requests.insert(
            commitment,
            InFlight {
                validation,
                deadline,
                view,
                proposal,
                overdue: false,
            },
        );
    }

    fn parent_in_flight(&self, commitment: &Commitment<Leaf2<T>>) -> bool {
        self.state_requests
            .get(commitment)
            .is_some_and(|in_flight| !in_flight.overdue)
    }

    pub fn request_header(&mut self, request: HeaderRequest<T>) {
        let parent_commitment = proposal_commitment(&request.parent_proposal);
        if self
            .header_requests
            .contains_key(&(request.view, parent_commitment))
        {
            return;
        }

        if self.parent_in_flight(&parent_commitment) {
            self.pending_requests
                .entry(parent_commitment)
                .or_default()
                .push(Pending::Header(request));
            return;
        }

        let Some(parent_entry) = self.validated_states.get(&parent_commitment).cloned() else {
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

        let Ok(version) = self.upgrade_lock.version(view) else {
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

        self.header_requests
            .insert((view, parent_commitment), handle);
    }

    /// Provide an externally-obtained validated state.
    pub fn update_state(&mut self, update: UpdateLeaf<T>) {
        let UpdateLeaf {
            view,
            leaf,
            state,
            delta,
        } = update;
        let commitment = leaf.commit();
        self.insert_state(view, state, delta, leaf);
        if let Some(in_flight) = self.state_requests.remove(&commitment) {
            in_flight.abort();
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
                        leaf,
                        validated,
                    } => {
                        let Some(in_flight) = self.state_requests.remove(&response.commitment)
                        else {
                            continue;
                        };
                        in_flight.deadline.abort();
                        // A failed validation still leaves a stub so queued
                        // requests proceed via catchup.
                        let measurement = if validated {
                            self.update_leaf_duration_metric
                                .clone()
                                .map(Measurement::start)
                        } else {
                            None
                        };
                        self.insert_state(
                            response.view,
                            response.state.clone(),
                            response.delta.clone(),
                            leaf,
                        );
                        finish_measurement(measurement);
                        self.start_pending(response.commitment);
                        return Some(StateManagerOutput::State {
                            response,
                            validated,
                        });
                    },
                    Completed::Header { response, header } => {
                        let key = (
                            response.view,
                            proposal_commitment(&response.parent_proposal),
                        );
                        if self.header_requests.remove(&key).is_none() {
                            continue;
                        }
                        return Some(StateManagerOutput::Header { response, header });
                    },
                    Completed::Deadline(proposal) => {
                        let commitment = proposal_commitment(&proposal);
                        let Some(in_flight) = self.state_requests.get_mut(&commitment) else {
                            continue;
                        };
                        in_flight.overdue = true;
                        warn!(
                            view = %proposal.view_number(),
                            deadline = ?self.parent_deadline,
                            "state validation still running past the parent deadline; \
                             descendants proceed against a from_header stub"
                        );
                        self.seed_from_header(proposal);
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

    /// The decided view's own validation, or a header this node needs to
    /// propose, may be queued behind a validation aborted here. A stub for the
    /// aborted proposal lets them proceed via catchup instead of hanging.
    pub fn gc(&mut self, view_number: ViewNumber) {
        self.validated_states
            .retain(|_, entry| entry.leaf.view_number() >= view_number);

        self.header_requests.retain(|(view, _), handle| {
            let keep = *view >= view_number;
            if !keep {
                handle.abort();
            }
            keep
        });

        self.pending_requests.retain(|_, pending| {
            pending.retain(|p| p.view() >= view_number);
            !pending.is_empty()
        });

        let stale: Vec<_> = self
            .state_requests
            .extract_if(|_, in_flight| in_flight.view < view_number)
            .collect();
        for (commitment, in_flight) in stale {
            in_flight.abort();
            if self.pending_requests.contains_key(&commitment) {
                self.seed_from_header(in_flight.proposal);
            }
        }
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
        if let Some(existing) = self.validated_states.get(&leaf.commit())
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
            .insert(leaf.commit(), StateEntry { state, delta, leaf });
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
        self.validated_states
            .iter()
            .any(|(_, entry)| entry.leaf.view_number() == v)
    }

    #[cfg(test)]
    pub(crate) fn pending_contains_commitment(&self, c: &Commitment<Leaf2<T>>) -> bool {
        self.pending_requests.contains_key(c)
    }
}
