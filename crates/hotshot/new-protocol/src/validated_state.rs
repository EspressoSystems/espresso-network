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

pub(crate) struct ValidatedStateManager<TYPES: NodeType> {
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

    /// Seed the manager with a validated state at a given view.
    pub(crate) fn seed_state(
        &mut self,
        view: ViewNumber,
        state: Arc<TYPES::ValidatedState>,
        leaf: Leaf2<TYPES>,
    ) {
        self.validated_states.insert(view, (state, leaf));
    }

    pub(crate) async fn run(mut self) {
        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await;
                },
                Some(completed_request) = self.completed_requests_rx.recv() => {
                    self.handle_completed_request(completed_request).await;
                },
                else => break,
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

    fn start_pending(
        &mut self,
        finished_commitment: Commitment<Leaf2<TYPES>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let Some(pending_requests) = self.pending_requests.remove(&finished_commitment) else {
                return;
            };
            for event in pending_requests {
                self.handle_event(event).await;
            }
        })
    }

    async fn handle_update_state(
        &mut self,
        state: TYPES::ValidatedState,
        view: ViewNumber,
        leaf: Leaf2<TYPES>,
    ) {
        let commitment = leaf.commit();
        self.validated_states.insert(view, (Arc::new(state), leaf));
        if let Some((handle, _)) = self.in_progress_requests.remove(&commitment) {
            // we have the state for this commitment so we cancel the
            // in progress request for this commitment.
            handle.abort();
        }
        self.start_pending(commitment).await;
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
    use tokio::sync::mpsc;

    use super::*;
    use crate::{
        events::{ConsensusOutput, HeaderRequest, StateRequest, Event},
        helpers::proposal_commitment,
        tests::test_utils::{TestData, TestView},
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

    struct StateTestHarness {
        manager: ValidatedStateManager<TestTypes>,
        event_rx: mpsc::Receiver<ConsensusOutput<TestTypes>>,
    }

    impl StateTestHarness {
        async fn new() -> Self {
            let (_, state_rx) = mpsc::channel(100);
            let (event_tx, event_rx) = mpsc::channel(100);
            let coordinator_handle = CoordinatorHandle::new(event_tx);
            let manager = ValidatedStateManager::new(
                state_rx,
                Arc::new(TestInstanceState::default()),
                coordinator_handle,
            );
            Self { manager, event_rx }
        }

        /// Seed the manager with the genesis state at view 0.
        async fn seed_genesis(&mut self) {
            let genesis_state = TestValidatedState::default();
            let genesis_leaf = Leaf2::<TestTypes>::genesis(
                &genesis_state,
                &TestInstanceState::default(),
                TEST_VERSIONS.test.base,
            )
            .await;
            self.manager
                .seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);
        }

        async fn request_state(&mut self, request: StateRequest<TestTypes>) {
            self.manager.handle_request_state(request).await;
        }

        async fn request_header(&mut self, request: HeaderRequest<TestTypes>) {
            self.manager.handle_request_header(request).await;
        }

        /// Wait for spawned tasks to complete and process their results.
        async fn process_completions(&mut self) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            while let Ok(completed) = self.manager.completed_requests_rx.try_recv() {
                self.manager.handle_completed_request(completed).await;
            }
        }

        fn collect_events(&mut self) -> Vec<ConsensusOutput<TestTypes>> {
            let mut events = Vec::new();
            while let Ok(event) = self.event_rx.try_recv() {
                events.push(event);
            }
            events
        }

        fn count_state_verified(events: &[ConsensusOutput<TestTypes>]) -> usize {
            events
                .iter()
                .filter(|e| matches!(e, ConsensusOutput::Event(Event::StateVerified(_))))
                .count()
        }

        fn count_header_created(events: &[ConsensusOutput<TestTypes>]) -> usize {
            events
                .iter()
                .filter(|e| matches!(e, ConsensusOutput::Event(Event::HeaderCreated(_, _))))
                .count()
        }
    }

    /// State request with missing parent inserts empty state (no response sent).
    #[tokio::test]
    async fn test_state_request_missing_parent_inserts_empty() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(2).await;

        // View 1's parent is genesis (view 0), which isn't seeded.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.process_completions().await;

        let events = harness.collect_events();
        // No StateVerified response because parent was missing — empty state inserted.
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            0,
            "No state response when parent is missing"
        );
        // But the empty state should be stored for the view.
        assert!(
            harness
                .manager
                .validated_states
                .contains_key(&test_data.views[0].view_number),
            "Empty state should be inserted for the view"
        );
    }

    /// State request with seeded genesis parent spawns validation and sends response.
    #[tokio::test]
    async fn test_state_request_with_genesis_parent() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(2).await;

        harness.seed_genesis().await;

        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.process_completions().await;

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            1,
            "Should receive StateVerified after validation completes"
        );
    }

    /// Sequential state requests: view 1 completes, then view 2 uses its result.
    #[tokio::test]
    async fn test_sequential_state_requests() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Request view 1 and let it complete.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.process_completions().await;

        // Request view 2 — parent (view 1) should now exist.
        harness
            .request_state(make_state_request(&test_data.views[1]))
            .await;
        harness.process_completions().await;

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            2,
            "Both views should produce StateVerified"
        );
    }

    /// State request queued behind in-progress parent auto-starts when parent completes.
    #[tokio::test]
    async fn test_state_request_queued_behind_parent() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Send both requests before either completes.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness
            .request_state(make_state_request(&test_data.views[1]))
            .await;

        // View 2 should be queued as pending (parent view 1 is in progress).
        let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.proposal);
        assert!(
            harness
                .manager
                .pending_requests
                .contains_key(&view_1_commit),
            "View 2 should be pending on view 1's commitment"
        );

        // Now let completions run — view 1 completes, which should start view 2.
        harness.process_completions().await;
        // Process again for view 2's completion.
        harness.process_completions().await;

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            2,
            "Both views should complete after pending resolution"
        );
    }

    /// Header request with existing parent state sends header response.
    #[tokio::test]
    async fn test_header_request_with_parent() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Complete state for view 1 so it can be used as parent for header.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.process_completions().await;
        // Drain the state verified event.
        harness.collect_events();

        // Now request a header with view 1 as parent.
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        harness.request_header(header_req).await;
        harness.process_completions().await;

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_header_created(&events),
            1,
            "Should receive HeaderCreated after header creation completes"
        );
    }

    /// Header request queued behind in-progress state starts when state completes.
    #[tokio::test]
    async fn test_header_request_queued_behind_state() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Send state request for view 1 (starts validation).
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;

        // Send header request with view 1 as parent BEFORE view 1 completes.
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        harness.request_header(header_req).await;

        // Header should be pending on view 1's commitment.
        let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.proposal);
        assert!(
            harness
                .manager
                .pending_requests
                .contains_key(&view_1_commit),
            "Header should be pending on view 1's commitment"
        );

        // Let view 1 complete — should also start the pending header.
        harness.process_completions().await;
        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            1,
            "State should be verified"
        );

        // Process again for the header task.
        harness.process_completions().await;

        let events = harness.collect_events();

        assert_eq!(
            StateTestHarness::count_header_created(&events),
            1,
            "Header should be created after pending state resolves"
        );
    }

    /// Duplicate state request for the same view is ignored.
    #[tokio::test]
    async fn test_duplicate_state_request_ignored() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(2).await;

        harness.seed_genesis().await;

        // Send same state request twice.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.process_completions().await;

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            1,
            "Duplicate request should be ignored — only one response"
        );
    }

    /// State and header requests for different views can be interleaved.
    #[tokio::test]
    async fn test_interleaved_state_and_header_requests() {
        let mut harness = StateTestHarness::new().await;
        let test_data = TestData::new(4).await;

        harness.seed_genesis().await;

        // Start state validation for views 1 and send header request for view 2
        // (with view 1 as parent) simultaneously.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness
            .request_state(make_state_request(&test_data.views[1]))
            .await;
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        harness.request_header(header_req).await;

        // Process all completions (may need multiple rounds).
        for _ in 0..3 {
            harness.process_completions().await;
        }

        let events = harness.collect_events();
        assert_eq!(
            StateTestHarness::count_state_verified(&events),
            2,
            "Both state requests should complete"
        );
        assert_eq!(
            StateTestHarness::count_header_created(&events),
            1,
            "Header request should complete"
        );
    }
}
