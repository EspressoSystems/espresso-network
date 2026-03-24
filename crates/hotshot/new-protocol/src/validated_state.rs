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
use tokio::task::{AbortHandle, JoinSet};
use tracing::error;

use crate::{
    events::{Event, HeaderRequest, StateRequest, StateResponse},
    helpers::{proposal_commitment, upgrade_lock},
};

type StateError<T> = <<T as NodeType>::ValidatedState as ValidatedState<T>>::Error;
type HeaderError<T> = <<T as NodeType>::BlockHeader as BlockHeader<T>>::Error;

pub(crate) struct ValidatedStateManager<T: NodeType> {
    instance: Arc<T::InstanceState>,
    validated_states: BTreeMap<ViewNumber, (Arc<T::ValidatedState>, Leaf2<T>)>,
    state_requests: HashMap<Commitment<Leaf2<T>>, (AbortHandle, StateRequest<T>)>,
    header_requests: HashMap<ViewNumber, AbortHandle>,
    pending_requests: HashMap<Commitment<Leaf2<T>>, Vec<Pending<T>>>,
    tasks: JoinSet<Completed<T>>,
}

enum Completed<T: NodeType> {
    State {
        commitment: Commitment<Leaf2<T>>,
        result: Result<StateResponse<T>, StateError<T>>,
    },
    Header {
        view: ViewNumber,
        result: Result<T::BlockHeader, HeaderError<T>>,
    },
}

enum Pending<T: NodeType> {
    State(StateRequest<T>),
    Header(HeaderRequest<T>),
}

impl<T: NodeType> ValidatedStateManager<T> {
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

    pub fn seed_state(&mut self, view: ViewNumber, state: Arc<T::ValidatedState>, leaf: Leaf2<T>) {
        self.validated_states.insert(view, (state, leaf));
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

        let Some((parent_state, parent_leaf)) =
            self.validated_states.get(&request.parent_view).cloned()
        else {
            self.insert_empty_state(request.proposal);
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
            let result = parent_state
                .validate_and_apply_header(
                    &instance,
                    &parent_leaf,
                    &header,
                    payload_size,
                    upgrade_lock,
                    *view,
                )
                .await
                .map(|(state, _delta)| StateResponse {
                    view,
                    commitment,
                    state: Arc::new(state),
                });
            Completed::State { commitment, result }
        });

        self.state_requests.insert(commitment, (handle, request));
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
        let Some((parent_state, parent_leaf)) = self.validated_states.get(&parent_view).cloned()
        else {
            error!(view = %request.view, "parent state not found for header request");
            return;
        };

        let instance = self.instance.clone();
        let view = request.view;

        let Ok(upgrade_lock) = upgrade_lock::<T>().version(view) else {
            error!(%view, "unsupported version");
            return;
        };

        let handle = self.tasks.spawn(async move {
            let result = T::BlockHeader::new(
                &parent_state,
                &instance,
                &parent_leaf,
                request.payload_commitment,
                request.builder_commitment,
                request.metadata,
                request.builder_fee,
                upgrade_lock,
                *view,
            )
            .await;
            Completed::Header { view, result }
        });

        self.header_requests.insert(view, handle);
    }

    /// Provide an externally-obtained validated state.
    pub fn update_state(&mut self, state: T::ValidatedState, view: ViewNumber, leaf: Leaf2<T>) {
        let commitment = leaf.commit();
        self.validated_states.insert(view, (Arc::new(state), leaf));
        if let Some((abort_handle, _)) = self.state_requests.remove(&commitment) {
            abort_handle.abort();
        }
        self.start_pending(commitment);
    }

    /// Wait for the next event.
    pub async fn next(&mut self) -> Option<Event<T>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => match result {
                    Completed::State { commitment, result } => {
                        if let Some(event) = self.handle_state_result(commitment, result) {
                            return Some(event);
                        }
                    },
                    Completed::Header { view, result } => {
                        if let Some(event) = self.handle_header_result(view, result) {
                            return Some(event);
                        }
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

    fn handle_state_result(
        &mut self,
        commitment: Commitment<Leaf2<T>>,
        result: Result<StateResponse<T>, StateError<T>>,
    ) -> Option<Event<T>> {
        let (_, request) = self.state_requests.remove(&commitment)?;
        match result {
            Ok(response) => {
                let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper::<T> {
                    proposal: request.proposal.clone(),
                });
                self.validated_states
                    .insert(response.view, (response.state, leaf));
                self.start_pending(response.commitment);
                Some(Event::StateVerified(request))
            },
            Err(err) => {
                error!(%err, "state validation failed");
                // Remove dependents of this failed request. TODO: double-check
                self.pending_requests.remove(&commitment);
                None
            },
        }
    }

    fn handle_header_result(
        &mut self,
        view: ViewNumber,
        result: Result<T::BlockHeader, HeaderError<T>>,
    ) -> Option<Event<T>> {
        self.header_requests.remove(&view)?;
        match result {
            Ok(header) => Some(Event::HeaderCreated(view, header)),
            Err(err) => {
                error!(%err, "header creation failed");
                None
            },
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

    fn insert_empty_state(&mut self, proposal: QuorumProposal2<T>) {
        let state = T::ValidatedState::from_header(&proposal.block_header);
        self.validated_states.insert(
            proposal.view_number(),
            (
                Arc::new(state),
                Leaf2::from_quorum_proposal(&QuorumProposalWrapper::<T> { proposal }),
            ),
        );
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use hotshot::traits::BlockPayload;
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestMetadata},
        node_types::{TEST_VERSIONS, TestTypes},
        state_types::{TestInstanceState, TestValidatedState},
    };
    use hotshot_types::{
        data::{Leaf2, vid_commitment},
        traits::{
            EncodeBytes,
            block_contents::{BlockHeader, BuilderFee},
            signature_key::BuilderSignatureKey,
        },
        vote::{Certificate, HasViewNumber},
    };

    use super::*;
    use crate::{
        events::{Event, HeaderRequest, StateRequest},
        helpers::proposal_commitment,
        tests::common::utils::{TestData, TestView},
    };

    /// Build a StateRequest from a TestView.
    fn make_state_request(view: &TestView) -> StateRequest<TestTypes> {
        let proposal = &view.proposal.data.proposal;
        StateRequest {
            view: view.view_number,
            parent_view: proposal.justify_qc.view_number(),
            epoch: view.epoch_number,
            block_number: BlockHeader::<TestTypes>::block_number(&proposal.block_header),
            proposal: proposal.clone(),
            parent_commitment: proposal.justify_qc.data().leaf_commit,
            payload_size: 0,
        }
    }

    /// Build a HeaderRequest from a TestView (as the parent).
    fn make_header_request(
        parent_view: &TestView,
        target_view: ViewNumber,
    ) -> HeaderRequest<TestTypes> {
        let parent_proposal = &parent_view.proposal.data.proposal;
        let block = TestBlockPayload::genesis();
        let metadata = TestMetadata {
            num_transactions: 0,
        };
        let payload_commitment = vid_commitment(
            &block.encode(),
            &metadata.encode(),
            10,
            TEST_VERSIONS.test.base,
        );
        let builder_commitment =
            <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&block, &metadata);
        let (builder_key, builder_private_key) =
            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let builder_signature =
            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::sign_builder_message(
                &builder_private_key,
                &[0u8],
            )
            .unwrap();
        HeaderRequest {
            view: target_view,
            epoch: parent_view.epoch_number,
            parent_proposal: parent_proposal.clone(),
            payload_commitment,
            builder_commitment,
            metadata,
            builder_fee: BuilderFee {
                fee_amount: 0,
                fee_account: builder_key,
                fee_signature: builder_signature,
            },
        }
    }

    async fn new_manager() -> ValidatedStateManager<TestTypes> {
        let mut manager = ValidatedStateManager::new(Arc::new(TestInstanceState::default()));
        let genesis_state = TestValidatedState::default();
        let genesis_leaf = Leaf2::<TestTypes>::genesis(
            &genesis_state,
            &TestInstanceState::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);
        manager
    }

    fn count_state_verified(events: &[Event<TestTypes>]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, Event::StateVerified(_)))
            .count()
    }

    fn count_header_created(events: &[Event<TestTypes>]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, Event::HeaderCreated(_, _)))
            .count()
    }

    /// State request with missing parent inserts empty state (no output produced).
    #[tokio::test]
    async fn test_state_request_missing_parent_inserts_empty() {
        let mut manager = ValidatedStateManager::new(Arc::new(TestInstanceState::default()));
        let test_data = TestData::new(2).await;

        // View 1's parent is genesis (view 0), which isn't seeded.
        manager.request_state(make_state_request(&test_data.views[0]));

        // No task was spawned, so next() should return None.
        assert!(
            manager.next().await.is_none(),
            "No output when parent is missing"
        );

        // But the empty state should be stored for the view.
        assert!(
            manager
                .validated_states
                .contains_key(&test_data.views[0].view_number),
            "Empty state should be inserted for the view"
        );
    }

    /// State request with seeded genesis parent spawns validation and produces output.
    #[tokio::test]
    async fn test_state_request_with_genesis_parent() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(2).await;

        manager.request_state(make_state_request(&test_data.views[0]));

        let output = manager.next().await.expect("should produce output");
        assert!(
            matches!(output, Event::StateVerified(_)),
            "Should receive StateVerified after validation completes"
        );
    }

    /// Sequential state requests: view 1 completes, then view 2 uses its result.
    #[tokio::test]
    async fn test_sequential_state_requests() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(3).await;

        // Request view 1 and let it complete.
        manager.request_state(make_state_request(&test_data.views[0]));
        manager.next().await.expect("view 1 should complete");

        // Request view 2 — parent (view 1) should now exist.
        manager.request_state(make_state_request(&test_data.views[1]));
        let output = manager.next().await.expect("should produce output");
        assert!(
            matches!(output, Event::StateVerified(_)),
            "View 2 should produce StateVerified"
        );
    }

    /// State request queued behind in-progress parent auto-starts when parent completes.
    #[tokio::test]
    async fn test_state_request_queued_behind_parent() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(3).await;

        // Send both requests before either completes.
        manager.request_state(make_state_request(&test_data.views[0]));
        manager.request_state(make_state_request(&test_data.views[1]));

        // View 2 should be queued as pending (parent view 1 is in progress).
        let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.proposal);
        assert!(
            manager.pending_requests.contains_key(&view_1_commit),
            "View 2 should be pending on view 1's commitment"
        );

        // next() should process view 1, then eagerly chain view 2.
        let output1 = manager.next().await.expect("view 1 should complete");
        let output2 = manager.next().await.expect("view 2 should complete");
        assert_eq!(
            count_state_verified(&[output1, output2]),
            2,
            "Both views should complete after pending resolution"
        );
    }

    /// Header request with existing parent state produces header output.
    #[tokio::test]
    async fn test_header_request_with_parent() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(3).await;

        // Complete state for view 1 so it can be used as parent for header.
        manager.request_state(make_state_request(&test_data.views[0]));
        manager.next().await.expect("view 1 should complete");

        // Now request a header with view 1 as parent.
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        manager.request_header(header_req);

        let output = manager.next().await.expect("should produce output");
        assert!(
            matches!(output, Event::HeaderCreated(_, _)),
            "Should receive HeaderCreated after header creation completes"
        );
    }

    /// Header request queued behind in-progress state starts when state completes.
    #[tokio::test]
    async fn test_header_request_queued_behind_state() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(3).await;

        // Send state request for view 1 (starts validation).
        manager.request_state(make_state_request(&test_data.views[0]));

        // Send header request with view 1 as parent BEFORE view 1 completes.
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        manager.request_header(header_req);

        // Header should be pending on view 1's commitment.
        let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.proposal);
        assert!(
            manager.pending_requests.contains_key(&view_1_commit),
            "Header should be pending on view 1's commitment"
        );

        // next() processes state completion, which chains the header request.
        let output1 = manager.next().await.expect("state should complete");
        assert!(
            matches!(output1, Event::StateVerified(_)),
            "State should be verified first"
        );

        let output2 = manager.next().await.expect("header should complete");
        assert!(
            matches!(output2, Event::HeaderCreated(_, _)),
            "Header should be created after pending state resolves"
        );
    }

    /// Duplicate state request for the same view is ignored.
    #[tokio::test]
    async fn test_duplicate_state_request_ignored() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(2).await;

        // Send same state request twice.
        manager.request_state(make_state_request(&test_data.views[0]));
        manager.request_state(make_state_request(&test_data.views[0]));

        let output = manager.next().await.expect("should produce output");
        assert!(matches!(output, Event::StateVerified(_)));

        // No second output — duplicate was ignored.
        assert!(
            manager.next().await.is_none(),
            "Duplicate request should be ignored — only one response"
        );
    }

    /// State and header requests for different views can be interleaved.
    #[tokio::test]
    async fn test_interleaved_state_and_header_requests() {
        let mut manager = new_manager().await;
        let test_data = TestData::new(4).await;

        // Start state validation for views 1 and 2, plus header request for view 2
        // (with view 1 as parent).
        manager.request_state(make_state_request(&test_data.views[0]));
        manager.request_state(make_state_request(&test_data.views[1]));
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        manager.request_header(header_req);

        // Collect all outputs.
        let mut outputs = Vec::new();
        for _ in 0..3 {
            outputs.push(manager.next().await.expect("should produce output"));
        }

        assert_eq!(
            count_state_verified(&outputs),
            2,
            "Both state requests should complete"
        );
        assert_eq!(
            count_header_created(&outputs),
            1,
            "Header request should complete"
        );
    }
}
