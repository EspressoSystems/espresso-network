use crate::state_signature::{
    LightClientState, StakeTableCommitmentType, StateKeyPair, StateSignature, StateSignatureScheme,
};
use async_std::sync::{Arc, RwLock};
use derivative::Derivative;
use hotshot::types::SystemContextHandle;
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_types::light_client::StateSignatureRequestBody;
use hotshot_types::light_client::StateVerKey;
use jf_primitives::signatures::SignatureScheme;
use surf_disco::Client;
use tide_disco::error::ServerError;
use url::Url;

use crate::{
    network,
    state_signature::{self, StateSignatureMemStorage},
    Node, SeqTypes,
};

/// The consensus handle
pub type Consensus<N> = SystemContextHandle<SeqTypes, Node<N>>;

/// The sequencer context contains a consensus handle and other sequencer specific information.
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct SequencerContext<N: network::Type> {
    /// The consensus handle
    handle: Consensus<N>,

    /// Index of this sequencer node
    #[allow(dead_code)]
    node_index: u64,

    /// Key pair for signing a new light client state
    state_key_pair: Arc<StateKeyPair>,

    /// The most recent light client state signatures
    state_signatures: Arc<RwLock<StateSignatureMemStorage>>,

    /// Commitment for current fixed stake table
    stake_table_comm: Arc<StakeTableCommitmentType>,

    /// The state relay server url
    state_relay_server_client: Option<Client<ServerError>>,

    /// An orchestrator to wait for before starting consensus.
    wait_for_orchestrator: Option<Arc<OrchestratorClient>>,
}

impl<N: network::Type> SequencerContext<N> {
    /// Constructor
    pub fn new(
        handle: Consensus<N>,
        node_index: u64,
        state_key_pair: StateKeyPair,
        stake_table_comm: StakeTableCommitmentType,
    ) -> Self {
        Self {
            handle,
            node_index,
            state_key_pair: Arc::new(state_key_pair),
            state_signatures: Default::default(),
            stake_table_comm: Arc::new(stake_table_comm),
            state_relay_server_client: None,
            wait_for_orchestrator: None,
        }
    }

    /// Wait for a signal from the orchestrator before starting consensus.
    pub fn wait_for_orchestrator(mut self, client: OrchestratorClient) -> Self {
        self.wait_for_orchestrator = Some(Arc::new(client));
        self
    }

    /// Connect to the given state relay server to send signed HotShot states to.
    pub fn with_state_relay_server(mut self, url: Url) -> Self {
        self.state_relay_server_client = Some(Client::new(url));
        self
    }

    /// Return a reference to the underlying consensus handle.
    pub fn consensus(&self) -> &Consensus<N> {
        &self.handle
    }

    /// Return a mutable reference to the underlying consensus handle.
    pub fn consensus_mut(&mut self) -> &mut Consensus<N> {
        &mut self.handle
    }

    /// Return a signature of a light client state at given height.
    pub async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        let pool_guard = self.state_signatures.read().await;
        pool_guard.get_signature(height)
    }

    /// Sign the light client state at given height and store it.
    pub async fn sign_new_state(&self, state: &LightClientState) -> StateSignature {
        let msg: [state_signature::FieldType; 7] = state.into();
        let signature = StateSignatureScheme::sign(
            &(),
            self.state_key_pair.sign_key_ref(),
            msg,
            &mut rand::thread_rng(),
        )
        .unwrap();
        let mut pool_guard = self.state_signatures.write().await;
        pool_guard.push(
            state.block_height as u64,
            StateSignatureRequestBody {
                key: self.get_state_ver_key(),
                state: state.clone(),
                signature: signature.clone(),
            },
        );
        tracing::debug!(
            "New signature added for block height {}",
            state.block_height
        );
        signature
    }

    /// Get the public key for light client state
    pub fn get_state_ver_key(&self) -> StateVerKey {
        self.state_key_pair.ver_key()
    }

    /// Return a commitment of the current fixed stake table
    pub fn get_stake_table_comm(&self) -> &StakeTableCommitmentType {
        &self.stake_table_comm
    }

    /// Return a url to the state relay server
    pub fn get_state_relay_server_client(&self) -> &Option<Client<ServerError>> {
        &self.state_relay_server_client
    }

    /// Start participating in consensus.
    pub async fn start_consensus(&self) {
        if let Some(orchestrator_client) = &self.wait_for_orchestrator {
            tracing::info!("waiting for orchestrated start");
            orchestrator_client
                .wait_for_all_nodes_ready(self.node_index)
                .await;
        }
        self.handle.hotshot.start_consensus().await;
    }
}
