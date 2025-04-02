//! Utilities for generating and storing the most recent light client state signatures.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use async_lock::RwLock;
use espresso_types::{traits::SequencerPersistence, PubKey};
use hotshot::types::{Event, EventType};
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::{
    data::EpochNumber,
    event::LeafInfo,
    light_client::{
        CircuitField, LightClientState, StakeTableState, StateSignKey, StateSignature,
        StateSignatureRequestBody, StateSignatureScheme, StateVerKey,
    },
    signature_key::BLSPubKey,
    traits::{
        block_contents::BlockHeader,
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
        signature_key::StakeTableEntryType,
        stake_table::StakeTableScheme as _,
    },
    utils::{epoch_from_block_number, is_last_block},
    PeerConfig,
};
use jf_signature::SignatureScheme;
use surf_disco::{Client, Url};
use tide_disco::error::ServerError;
use vbs::version::StaticVersionType;

use crate::{context::Consensus, SeqTypes};

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
    voting_stake_table: StakeTableState,

    /// Capacity of the stake table
    stake_table_capacity: u64,

    /// The state relay server url
    relay_server_client: Option<Client<ServerError, ApiVer>>,
}

impl<ApiVer: StaticVersionType> StateSigner<ApiVer> {
    pub fn new(
        sign_key: StateSignKey,
        ver_key: StateVerKey,
        voting_stake_table: StakeTableState,
        stake_table_capacity: u64,
    ) -> Self {
        Self {
            sign_key,
            ver_key,
            voting_stake_table,
            stake_table_capacity,
            signatures: Default::default(),
            relay_server_client: Default::default(),
        }
    }

    /// Connect to the given state relay server to send signed HotShot states to.
    pub fn with_relay_server(mut self, url: Url) -> Self {
        self.relay_server_client = Some(Client::new(url));
        self
    }

    pub(super) async fn handle_event<N, P, V>(
        &mut self,
        event: &Event<SeqTypes>,
        consensus_state: Arc<RwLock<Consensus<N, P, V>>>,
    ) where
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
        V: Versions,
    {
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
                tracing::debug!("New leaves decided. Latest block height: {}", leaf.height(),);

                let consensus = consensus_state.read().await;
                let cur_block_height = state.block_height;
                let blocks_per_epoch = consensus.epoch_height;

                let next_stake_table = if is_last_block(cur_block_height, blocks_per_epoch) {
                    // during the last block of each epoch, we will use a new `next_stake_table`
                    let cur_epoch = epoch_from_block_number(cur_block_height, blocks_per_epoch);
                    let Ok(membership) = consensus
                        .membership_coordinator
                        .membership_for_epoch(Some(EpochNumber::new(cur_epoch + 1)))
                        .await
                    else {
                        tracing::error!("Fail to get membership for epoch: {}", cur_epoch + 1);
                        return;
                    };
                    compute_stake_table_commitment(
                        &membership.stake_table().await,
                        self.stake_table_capacity as usize,
                    )
                } else {
                    // during non-last-block (most cases), the stake table used for the next block is exactly the same
                    self.voting_stake_table
                };

                let signature = self.sign_new_state(&state, next_stake_table).await;

                if let Some(client) = &self.relay_server_client {
                    let request_body = StateSignatureRequestBody {
                        key: self.ver_key.clone(),
                        state,
                        next_stake: next_stake_table,
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

                // update the voting stake table for future blocks
                self.voting_stake_table = next_stake_table;
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
    async fn sign_new_state(
        &self,
        state: &LightClientState,
        next_stake_table: StakeTableState,
    ) -> StateSignature {
        let mut msg = Vec::with_capacity(7);
        let state_msg: [CircuitField; 3] = state.into();
        msg.extend_from_slice(&state_msg);
        let next_stake_msg: [CircuitField; 4] = next_stake_table.into();
        msg.extend_from_slice(&next_stake_msg);

        let signature =
            StateSignatureScheme::sign(&(), &self.sign_key, msg, &mut rand::thread_rng()).unwrap();
        let mut pool_guard = self.signatures.write().await;
        pool_guard.push(
            state.block_height,
            StateSignatureRequestBody {
                key: self.ver_key.clone(),
                state: state.clone(),
                next_stake: next_stake_table,
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

/// Given a list of stakers from `PeerConfig`, compute the stake table commitment
pub fn compute_stake_table_commitment(
    known_nodes_with_stakes: &[PeerConfig<SeqTypes>],
    capacity: usize,
) -> StakeTableState {
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
    st.voting_state().unwrap() // safe unwrap
}
