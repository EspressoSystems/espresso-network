//! Utilities for generating and storing the most recent light client state signatures.

use std::collections::{HashMap, VecDeque};

use async_lock::RwLock;
use hotshot::types::{Event, EventType};
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::{
    event::LeafInfo,
    light_client::{
        CircuitField, LightClientState, StateSignKey, StateSignature, StateSignatureRequestBody,
        StateSignatureScheme, StateVerKey,
    },
    signature_key::BLSPubKey,
    traits::{
        block_contents::BlockHeader,
        signature_key::StakeTableEntryType,
        stake_table::{SnapshotVersion, StakeTableScheme as _},
    },
    PeerConfig,
};
use jf_signature::SignatureScheme;
use surf_disco::{Client, Url};
use tide_disco::error::ServerError;
use vbs::version::StaticVersionType;

use crate::SeqTypes;

/// A relay server that's collecting and serving the light client state signatures
pub mod relay_server;

/// Capacity for the in memory signature storage.
const SIGNATURE_STORAGE_CAPACITY: usize = 100;

#[derive(Debug)]
pub struct StateSigner<ApiVer: StaticVersionType> {
    /// Key for signing a new light client state
    sign_key: StateSignKey,

    /// Key for verifying a light client state
    ver_key: StateVerKey,

    /// The most recent light client state signatures
    signatures: RwLock<StateSignatureMemStorage>,

    /// Commitment for current fixed stake table
    #[allow(dead_code)] // although not used today, might need it for dynamic stake table later
    stake_table_comm: StakeTableCommitmentType,

    /// The state relay server url
    relay_server_client: Option<Client<ServerError, ApiVer>>,
}

impl<ApiVer: StaticVersionType> StateSigner<ApiVer> {
    pub fn new(
        sign_key: StateSignKey,
        ver_key: StateVerKey,
        stake_table_comm: StakeTableCommitmentType,
    ) -> Self {
        Self {
            sign_key,
            ver_key,
            stake_table_comm,
            signatures: Default::default(),
            relay_server_client: Default::default(),
        }
    }

    /// Connect to the given state relay server to send signed HotShot states to.
    pub fn with_relay_server(mut self, url: Url) -> Self {
        self.relay_server_client = Some(Client::new(url));
        self
    }

    pub(super) async fn handle_event(&self, event: &Event<SeqTypes>) {
        let EventType::Decide { leaf_chain, .. } = &event.event else {
            return;
        };
        let Some(LeafInfo { leaf, .. }) = leaf_chain.first() else {
            return;
        };
        match leaf
            .block_header()
            .get_light_client_state(leaf.view_number())
        {
            Ok(state) => {
                let signature = self.sign_new_state(&state).await;
                tracing::debug!("New leaves decided. Latest block height: {}", leaf.height(),);

                if let Some(client) = &self.relay_server_client {
                    let request_body = StateSignatureRequestBody {
                        key: self.ver_key.clone(),
                        state,
                        signature,
                    };
                    if let Err(error) = client
                        .post::<()>("api/state")
                        .body_binary(&request_body)
                        .unwrap()
                        .send()
                        .await
                    {
                        tracing::warn!("Error posting signature to the relay server: {:?}", error);
                    }
                }
            },
            Err(err) => {
                tracing::error!("Error generating light client state: {:?}", err)
            },
        }
    }

    /// Return a signature of a light client state at given height.
    pub async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        let pool_guard = self.signatures.read().await;
        pool_guard.get_signature(height)
    }

    /// Sign the light client state at given height and store it.
    async fn sign_new_state(&self, state: &LightClientState) -> StateSignature {
        let msg: [CircuitField; 3] = state.into();
        let signature =
            StateSignatureScheme::sign(&(), &self.sign_key, msg, &mut rand::thread_rng()).unwrap();
        let mut pool_guard = self.signatures.write().await;
        pool_guard.push(
            state.block_height,
            StateSignatureRequestBody {
                key: self.ver_key.clone(),
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
}

/// A rolling in-memory storage for the most recent light client state signatures.
#[derive(Debug, Default)]
pub struct StateSignatureMemStorage {
    pool: HashMap<u64, StateSignatureRequestBody>,
    deque: VecDeque<u64>,
}

impl StateSignatureMemStorage {
    pub fn push(&mut self, height: u64, signature: StateSignatureRequestBody) {
        self.pool.insert(height, signature);
        self.deque.push_back(height);
        if self.pool.len() > SIGNATURE_STORAGE_CAPACITY {
            self.pool.remove(&self.deque.pop_front().unwrap());
        }
    }

    pub fn get_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        self.pool.get(&height).cloned()
    }
}

/// Type for stake table commitment
pub type StakeTableCommitmentType = (CircuitField, CircuitField, CircuitField);

/// Helper function for stake table commitment
pub fn static_stake_table_commitment(
    known_nodes_with_stakes: &[PeerConfig<SeqTypes>],
    capacity: usize,
) -> (CircuitField, CircuitField, CircuitField) {
    let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(capacity);
    known_nodes_with_stakes.iter().for_each(|peer| {
        // This `unwrap()` won't fail unless number of entries exceeds `capacity`
        st.register(
            *peer.stake_table_entry.key(),
            peer.stake_table_entry.stake(),
            peer.state_ver_key.clone(),
        )
        .unwrap();
    });
    st.advance();
    st.advance();
    // This `unwrap()` won't fail
    st.commitment(SnapshotVersion::LastEpochStart).unwrap()
}
