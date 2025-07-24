// I can't "mark this sync", clippy
#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::borrow_interior_mutable_const)]

use std::{cell::LazyCell, sync::Arc, time::Duration};

use async_broadcast::Sender;
use committable::Commitment;
use hotshot::{
    rand::{thread_rng, Rng},
    types::{BLSPubKey, Event, EventType, SignatureKey},
};
use hotshot_builder_api::v0_1::{
    block_info::{AvailableBlockHeaderInputV1, AvailableBlockInfo},
    builder::BuildError,
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
};
use hotshot_example_types::{block_types::TestTransaction, node_types::TestTypes};
use hotshot_task_impls::builder::v0_1::BuilderClient;
use hotshot_types::{
    data::ViewNumber,
    traits::node_implementation::{ConsensusTime, NodeType},
};
use builder_shared::{
    block::{BlockId, BuilderStateId},
    error::Error,
    utils::BuilderKeys,
};
use tokio::spawn;
use url::Url;
use vbs::version::StaticVersion;

use crate::service::{GlobalState, ProxyGlobalState};

mod basic;
mod block_size;
mod finalization;
mod integration;

const MOCK_LEADER_KEYS: LazyCell<BuilderKeys<TestTypes>> =
    LazyCell::new(|| BLSPubKey::generated_from_seed_indexed([0; 32], 0));

fn sign(
    data: &[u8],
) -> <<TestTypes as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType {
    <<TestTypes as NodeType>::SignatureKey as SignatureKey>::sign(&MOCK_LEADER_KEYS.1, data)
        .unwrap()
}

// We need to extract the error strings by hand because BuildError doesn't implement Eq
fn assert_eq_generic_err(err: BuildError, expected_err: Error<TestTypes>) {
    let BuildError::Error(expected_err_str) = expected_err.into() else {
        panic!("Unexpected conversion of Error to BuildError");
    };
    println!("{err:#?}");
    let BuildError::Error(err_str) = err else {
        panic!("Unexpected BuildError by builder");
    };
    assert_eq!(expected_err_str, err_str);
}

/// Provides convenience functions on top of service API,
/// as well as routing requests through as many API surfaces as
/// possible to shake out more potential bugs
struct TestServiceWrapper {
    event_sender: Sender<Event<TestTypes>>,
    proxy_global_state: ProxyGlobalState<TestTypes>,
    client: BuilderClient<TestTypes>,
}

impl TestServiceWrapper {
    async fn new(
        global_state: Arc<GlobalState<TestTypes>>,
        event_stream_sender: Sender<Event<TestTypes>>,
    ) -> Self {
        let port = portpicker::pick_unused_port().unwrap();
        let url: Url = format!("http://localhost:{port}").parse().unwrap();
        let app = Arc::clone(&global_state).into_app().unwrap();
        spawn(app.serve(url.clone(), StaticVersion::<0, 1> {}));
        let client = BuilderClient::new(url);
        assert!(client.connect(Duration::from_secs(1)).await);
        Self {
            event_sender: event_stream_sender,
            proxy_global_state: ProxyGlobalState(global_state),
            client,
        }
    }
}

impl TestServiceWrapper {
    /// Proxies request to get available blocks to underlying [`ProxyGlobalState`],
    /// taking care of signing
    pub(crate) async fn get_available_blocks(
        &self,
        state_id: &BuilderStateId<TestTypes>,
    ) -> Result<Vec<AvailableBlockInfo<TestTypes>>, BuildError> {
        self.proxy_global_state
            .available_blocks(
                &state_id.parent_commitment,
                *state_id.parent_view,
                MOCK_LEADER_KEYS.0,
                &BLSPubKey::sign(&MOCK_LEADER_KEYS.1, state_id.parent_commitment.as_ref()).unwrap(),
            )
            .await
    }

    /// Proxies request to get claim block header input to underlying [`ProxyGlobalState`],
    /// taking care of signing
    pub(crate) async fn claim_block_header_input(
        &self,
        block_id: &BlockId<TestTypes>,
    ) -> Result<AvailableBlockHeaderInputV1<TestTypes>, BuildError> {
        self.proxy_global_state
            .claim_block_header_input(
                &block_id.hash,
                *block_id.view,
                MOCK_LEADER_KEYS.0,
                &BLSPubKey::sign(&MOCK_LEADER_KEYS.1, block_id.hash.as_ref()).unwrap(),
            )
            .await
    }

    /// Combines get available block request and claim block request to
    /// get transactions a leader would've received for the round.
    ///
    /// Panics on errors.
    ///
    /// Requests are routed through HotShot's HTTP API client to check
    /// compatibility
    pub(crate) async fn get_transactions(
        &self,
        state_id: &BuilderStateId<TestTypes>,
    ) -> Vec<TestTransaction> {
        let mut available_states = self
            .client
            .available_blocks(
                state_id.parent_commitment,
                *state_id.parent_view,
                MOCK_LEADER_KEYS.0,
                &BLSPubKey::sign(&MOCK_LEADER_KEYS.1, state_id.parent_commitment.as_ref()).unwrap(),
            )
            .await
            .unwrap();

        let Some(block_info) = available_states.pop() else {
            return vec![];
        };

        // Get block for its transactions
        let block = self
            .client
            .claim_block(
                block_info.block_hash.clone(),
                *state_id.parent_view,
                MOCK_LEADER_KEYS.0,
                &BLSPubKey::sign(&MOCK_LEADER_KEYS.1, block_info.block_hash.as_ref()).unwrap(),
            )
            .await
            .unwrap();
        block.block_payload.transactions
    }

    /// Emulates submission of transactions through HotShot gossiping
    pub(crate) async fn submit_transactions_public(&self, transactions: Vec<TestTransaction>) {
        self.event_sender
            .broadcast(Event {
                // This field is never actually used in the builder, so we can just use
                // an arbitrary value
                view_number: ViewNumber::genesis(),
                event: EventType::Transactions { transactions },
            })
            .await
            .unwrap();
    }

    /// Submits transactions through private mempool interface of [`ProxyGlobalState`]
    pub(crate) async fn submit_transactions_private(
        &self,
        transactions: Vec<TestTransaction>,
    ) -> Result<Vec<Commitment<TestTransaction>>, BuildError> {
        self.proxy_global_state.submit_txns(transactions).await
    }

    /// Submits transactions randomly either through public or private mempool
    pub(crate) async fn submit_transactions(&self, transactions: Vec<TestTransaction>) {
        if thread_rng().gen() {
            self.submit_transactions_public(transactions).await
        } else {
            self.submit_transactions_private(transactions)
                .await
                .unwrap();
        }
    }
}
