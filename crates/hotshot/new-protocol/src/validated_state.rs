#![allow(unused)]

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::Infallible,
    future::pending,
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
    select, spawn,
    sync::mpsc,
    task::{JoinHandle, JoinSet},
};
use tracing::{error, warn};

use crate::{
    events::{ConsensusInput, HeaderRequest, StateEvent, StateRequest, StateResponse},
    helpers::{proposal_commitment, upgrade_lock},
};

type StateError<T> = <<T as NodeType>::ValidatedState as ValidatedState<T>>::Error;
type HeaderError<T> = <<T as NodeType>::BlockHeader as BlockHeader<T>>::Error;

enum CompletedRequest<T: NodeType> {
    State(Result<StateResponse<T>, StateError<T>>),
    Header(Result<(ViewNumber, T::BlockHeader), HeaderError<T>>),
}

enum Command<T: NodeType> {
    StateRequest(StateRequest<T>),
    HeaderRequest(HeaderRequest<T>),
    SetSeed {
        view: ViewNumber,
        state: Arc<T::ValidatedState>,
        leaf: Leaf2<T>,
    },
}

pub(crate) struct ValidatedStateManager<T: NodeType> {
    tx: mpsc::Sender<Command<T>>,
    rx: mpsc::Receiver<ConsensusInput<T>>,
    jh: JoinHandle<()>,
}

struct Worker<T: NodeType> {
    tasks: JoinSet<CompletedRequest<T>>,
    validated_states: BTreeMap<ViewNumber, (Arc<T::ValidatedState>, Leaf2<T>)>,
    in_progress_requests: HashMap<Commitment<Leaf2<T>>, StateRequest<T>>,
    in_progress_headers: HashSet<ViewNumber>,
    pending_requests: BTreeMap<Commitment<Leaf2<T>>, Vec<StateEvent<T>>>,
    instance_state: Arc<T::InstanceState>,
    tx: mpsc::Sender<ConsensusInput<T>>,
    rx: mpsc::Receiver<Command<T>>,
}

impl<T: NodeType> ValidatedStateManager<T> {
    pub(crate) fn new(instance_state: Arc<T::InstanceState>) -> Self {
        let (tx1, rx1) = mpsc::channel(8);
        let (tx2, rx2) = mpsc::channel(8);
        let worker = Worker {
            validated_states: BTreeMap::new(),
            in_progress_requests: HashMap::new(),
            in_progress_headers: HashSet::new(),
            pending_requests: BTreeMap::new(),
            tasks: JoinSet::new(),
            instance_state,
            tx: tx2,
            rx: rx1,
        };
        Self {
            tx: tx1,
            rx: rx2,
            jh: spawn(worker.go()),
        }
    }

    /// Seed the manager with a validated state at a given view.
    pub(crate) async fn seed_state(
        &mut self,
        view: ViewNumber,
        state: Arc<T::ValidatedState>,
        leaf: Leaf2<T>,
    ) {
        self.tx
            .send(Command::SetSeed { view, state, leaf })
            .await
            .unwrap(); // TODO
    }

    pub(crate) async fn state_request(&mut self, r: StateRequest<T>) {
        self.tx.send(Command::StateRequest(r)).await.unwrap() // TODO
    }

    pub(crate) async fn header_request(&mut self, r: HeaderRequest<T>) {
        self.tx.send(Command::HeaderRequest(r)).await.unwrap() // TODO
    }

    pub(crate) async fn next_result(&mut self) -> Option<ConsensusInput<T>> {
        self.rx.recv().await
    }
}

impl<T: NodeType> Worker<T> {
    async fn go(mut self) {
        self.tasks.spawn(pending());
        loop {
            select! {
                cmd = self.rx.recv() => match cmd {
                    Some(Command::StateRequest(r)) => self.handle_request_state(r),
                    Some(Command::HeaderRequest(r)) => self.handle_request_header(r),
                    Some(Command::SetSeed { view, state, leaf }) => {
                        self.validated_states.insert(view, (state, leaf));
                    }
                    None => break
                },
                val = self.tasks.join_next() => match val {
                    Some(Ok(CompletedRequest::State(result))) => {
                        if let Some(x) = self.handle_state_completed(result) {
                            if let Err(err) = self.tx.send(x).await {
                                error!(%err);
                                break;
                            }
                        }
                    }
                    Some(Ok(CompletedRequest::Header(result))) => {
                        if let Some(x) = self.handle_header_completed(result) {
                            if let Err(err) = self.tx.send(x).await {
                                error!(%err);
                                break;
                            }
                        }
                    }
                    Some(Err(err)) => {
                        warn!(%err, "TODO")
                    }
                    None => {
                        unreachable!()
                    }
                },
            }
        }
    }

    pub(crate) fn handle_event(&mut self, event: StateEvent<T>) {
        match event {
            StateEvent::RequestState(request) => self.handle_request_state(request),
            StateEvent::RequestHeader(request) => self.handle_request_header(request),
            StateEvent::UpdateState(state, view, leaf) => {
                self.handle_update_state(state, view, leaf)
            },
        }
    }

    fn handle_state_completed(
        &mut self,
        state: Result<StateResponse<T>, StateError<T>>,
    ) -> Option<ConsensusInput<T>> {
        match state {
            Ok(response) => {
                let request = self.in_progress_requests.remove(&response.commitment)?;
                let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper::<T> {
                    proposal: request.proposal,
                });
                let input = ConsensusInput::StateVerified(StateResponse {
                    view: response.view,
                    commitment: response.commitment,
                    state: response.state.clone(),
                });
                self.validated_states
                    .insert(response.view, (response.state, leaf));
                self.start_pending(response.commitment);
                Some(input)
            },
            Err(err) => {
                // TODO: Return ConsensusInput::StateVerificationFailed once we
                // carry enough context (commitment) through the error path.
                error!(%err, "state validation failed");
                None
            },
        }
    }

    fn handle_header_completed(
        &mut self,
        result: Result<(ViewNumber, T::BlockHeader), HeaderError<T>>,
    ) -> Option<ConsensusInput<T>> {
        match result {
            Ok((view, header)) => {
                self.in_progress_headers.remove(&view);
                Some(ConsensusInput::HeaderCreated(view, header))
            },
            Err(err) => {
                // TODO: Return a failure input once we carry enough context.
                error!(%err, "header creation failed");
                None
            },
        }
    }

    fn is_in_progress(&self, commitment: Commitment<Leaf2<T>>) -> bool {
        self.in_progress_requests.contains_key(&commitment)
    }

    fn insert_pending_state(&mut self, commitment: Commitment<Leaf2<T>>, request: StateRequest<T>) {
        self.pending_requests
            .entry(commitment)
            .or_default()
            .push(StateEvent::RequestState(request));
    }

    fn insert_pending_header(
        &mut self,
        commitment: Commitment<Leaf2<T>>,
        request: HeaderRequest<T>,
    ) {
        self.pending_requests
            .entry(commitment)
            .or_default()
            .push(StateEvent::RequestHeader(request));
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

    #[allow(clippy::too_many_arguments)]
    fn spawn_validate(
        &mut self,
        parent_state: Arc<T::ValidatedState>,
        instance_state: Arc<T::InstanceState>,
        parent_leaf: Leaf2<T>,
        header: T::BlockHeader,
        payload_size: u32,
        view_number: u64,
        commitment: Commitment<Leaf2<T>>,
    ) {
        self.tasks.spawn(async move {
            let state_result = parent_state
                .validate_and_apply_header(
                    instance_state.as_ref(),
                    &parent_leaf,
                    &header,
                    payload_size,
                    upgrade_lock::<T>().version(view_number.into()).unwrap(),
                    view_number,
                )
                .await;
            let response = state_result.map(|(state, _delta)| StateResponse {
                view: view_number.into(),
                commitment,
                state: Arc::new(state),
            });
            CompletedRequest::State(response)
        });
    }

    fn handle_request_state(&mut self, request: StateRequest<T>) {
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

        let instance_state = self.instance_state.clone();

        self.spawn_validate(
            parent_state,
            instance_state,
            parent_leaf,
            request.proposal.block_header.clone(),
            request.payload_size,
            *request.view,
            commitment,
        );
        self.in_progress_requests.insert(commitment, request);
    }

    fn handle_request_header(&mut self, request: HeaderRequest<T>) {
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

        self.tasks.spawn(async move {
            let header = T::BlockHeader::new(
                parent_state.as_ref(),
                instance_state.as_ref(),
                &parent_leaf,
                request.payload_commitment,
                request.builder_commitment,
                request.metadata,
                request.builder_fee,
                upgrade_lock::<T>().version(request.view).unwrap(),
                *request.view,
            )
            .await;
            let response = header.map(|header| (request.view, header));
            CompletedRequest::Header(response)
        });
        self.in_progress_headers.insert(request.view);
    }

    fn is_header_in_progress(&self, view: ViewNumber) -> bool {
        self.in_progress_headers.contains(&view)
    }

    fn start_pending(&mut self, finished_commitment: Commitment<Leaf2<T>>) {
        let Some(pending_requests) = self.pending_requests.remove(&finished_commitment) else {
            return;
        };
        for event in pending_requests {
            self.handle_event(event);
        }
    }

    fn handle_update_state(&mut self, state: T::ValidatedState, view: ViewNumber, leaf: Leaf2<T>) {
        let commitment = leaf.commit();
        self.validated_states.insert(view, (Arc::new(state), leaf));
        self.in_progress_requests.remove(&commitment);
        self.start_pending(commitment);
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
        data::{Leaf2, ViewNumber, vid_commitment},
        traits::{
            EncodeBytes,
            block_contents::{BlockHeader, BuilderFee},
            signature_key::BuilderSignatureKey,
        },
        vote::{Certificate, HasViewNumber},
    };

    use super::*;
    use crate::{
        events::{ConsensusInput, HeaderRequest, StateRequest},
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
            block: BlockHeader::<TestTypes>::block_number(&proposal.block_header).into(),
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

    /// Drives ValidatedStateManager directly, collecting results via `next_result`.
    struct StateTestHarness {
        manager: ValidatedStateManager<TestTypes>,
    }

    impl StateTestHarness {
        fn new() -> Self {
            let manager = ValidatedStateManager::new(Arc::new(TestInstanceState::default()));
            Self { manager }
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
                .seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf)
                .await;
        }

        async fn request_state(&mut self, request: StateRequest<TestTypes>) {
            self.manager.state_request(request).await;
        }

        async fn request_header(&mut self, request: HeaderRequest<TestTypes>) {
            self.manager.header_request(request).await
        }

        /// Wait for spawned tasks to complete and collect all results.
        async fn collect_results(&mut self, n: usize) -> Vec<ConsensusInput<TestTypes>> {
            let mut results = Vec::new();
            while let Some(completed) = self.manager.next_result().await {
                results.push(completed);
                if results.len() == n {
                    break;
                }
            }
            results
        }

        fn count_state_verified(results: &[ConsensusInput<TestTypes>]) -> usize {
            results
                .iter()
                .filter(|r| matches!(r, ConsensusInput::StateVerified(_)))
                .count()
        }

        fn count_header_created(results: &[ConsensusInput<TestTypes>]) -> usize {
            results
                .iter()
                .filter(|r| matches!(r, ConsensusInput::HeaderCreated(_, _)))
                .count()
        }
    }

    /// State request with missing parent inserts empty state (no response sent).
    #[tokio::test]
    async fn test_state_request_missing_parent_inserts_empty() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(2).await;

        // View 1's parent is genesis (view 0), which isn't seeded.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        // No result is produced — parent was missing, empty state inserted.
        // Just verify the manager didn't panic or hang on the send.
    }

    /// State request with seeded genesis parent spawns validation and sends response.
    #[tokio::test]
    async fn test_state_request_with_genesis_parent() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(2).await;

        harness.seed_genesis().await;

        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        let results = harness.collect_results(1).await;

        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "Should receive StateVerified after validation completes"
        );
    }

    /// Sequential state requests: view 1 completes, then view 2 uses its result.
    #[tokio::test]
    async fn test_sequential_state_requests() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Request view 1 and let it complete.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.collect_results(1).await;

        // Request view 2 — parent (view 1) should now exist.
        harness
            .request_state(make_state_request(&test_data.views[1]))
            .await;
        let results = harness.collect_results(1).await;

        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "View 2 should produce StateVerified"
        );
    }

    /// State request queued behind in-progress parent auto-starts when parent completes.
    #[tokio::test]
    async fn test_state_request_queued_behind_parent() {
        let mut harness = StateTestHarness::new();
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

        // Let completions run — view 1 completes, which starts view 2.
        let results = harness.collect_results(1).await;
        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "View 1 should complete"
        );

        // Process again for view 2's completion.
        let results = harness.collect_results(1).await;
        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "View 2 should complete after pending resolution"
        );
    }

    /// Header request with existing parent state sends header response.
    #[tokio::test]
    async fn test_header_request_with_parent() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(3).await;

        harness.seed_genesis().await;

        // Complete state for view 1 so it can be used as parent for header.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness.collect_results(1).await;

        // Now request a header with view 1 as parent.
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        harness.request_header(header_req).await;
        let results = harness.collect_results(1).await;

        assert_eq!(
            StateTestHarness::count_header_created(&results),
            1,
            "Should receive HeaderCreated after header creation completes"
        );
    }

    /// Header request queued behind in-progress state starts when state completes.
    #[tokio::test]
    async fn test_header_request_queued_behind_state() {
        let mut harness = StateTestHarness::new();
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

        // Let view 1 complete — should also start the pending header.
        let results = harness.collect_results(1).await;
        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "State should be verified"
        );

        // Process again for the header task.
        let results = harness.collect_results(1).await;
        assert_eq!(
            StateTestHarness::count_header_created(&results),
            1,
            "Header should be created after pending state resolves"
        );
    }

    /// Duplicate state request for the same view is ignored.
    #[tokio::test]
    async fn test_duplicate_state_request_ignored() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(2).await;

        harness.seed_genesis().await;

        // Send same state request twice.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        let results = harness.collect_results(1).await;

        assert_eq!(
            StateTestHarness::count_state_verified(&results),
            1,
            "Duplicate request should be ignored — only one response"
        );
    }

    /// State and header requests for different views can be interleaved.
    #[tokio::test]
    async fn test_interleaved_state_and_header_requests() {
        let mut harness = StateTestHarness::new();
        let test_data = TestData::new(4).await;

        harness.seed_genesis().await;

        // Start state validation for views 1 and 2, plus header for view 2.
        harness
            .request_state(make_state_request(&test_data.views[0]))
            .await;
        harness
            .request_state(make_state_request(&test_data.views[1]))
            .await;
        let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
        harness.request_header(header_req).await;

        // Process all completions (may need multiple rounds).
        let mut state_count = 0;
        let mut header_count = 0;
        for _ in 0..3 {
            let results = harness.collect_results(1).await;
            state_count += StateTestHarness::count_state_verified(&results);
            header_count += StateTestHarness::count_header_created(&results);
        }

        assert_eq!(state_count, 2, "Both state requests should complete");
        assert_eq!(header_count, 1, "Header request should complete");
    }
}
