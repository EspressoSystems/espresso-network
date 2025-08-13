//! Utilities for generating and storing the most recent light client state signatures.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use alloy::primitives::FixedBytes;
use async_lock::RwLock;
use espresso_types::{traits::SequencerPersistence, PubKey};
use hotshot::types::{Event, EventType, SchnorrPubKey};
use hotshot_task_impls::helpers::derive_signed_state_digest;
use hotshot_types::{
    event::LeafInfo,
    light_client::{
        LCV2StateSignatureRequestBody, LCV3StateSignatureRequestBody, LightClientState,
        StakeTableState, StateSignKey, StateSignature, StateVerKey,
    },
    traits::{
        block_contents::BlockHeader,
        network::ConnectedNetwork,
        node_implementation::{NodeType, Versions},
        signature_key::{LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey},
    },
    utils::{is_ge_epoch_root, option_epoch_from_block_number},
};
use jf_signature::SignatureError;
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

    /// epoch for the current stake table state
    voting_stake_table_epoch: Option<<SeqTypes as NodeType>::Epoch>,

    /// Capacity of the stake table
    stake_table_capacity: usize,

    /// The state relay server url
    relay_server_client: Option<Client<ServerError, ApiVer>>,
}

impl<ApiVer: StaticVersionType> StateSigner<ApiVer> {
    pub fn new(
        sign_key: StateSignKey,
        ver_key: StateVerKey,
        voting_stake_table: StakeTableState,
        voting_stake_table_epoch: Option<<SeqTypes as NodeType>::Epoch>,
        stake_table_capacity: usize,
    ) -> Self {
        Self {
            sign_key,
            ver_key,
            voting_stake_table,
            voting_stake_table_epoch,
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

                // The last few state updates are handled in the consensus, we do not sign them.
                if leaf.with_epoch & is_ge_epoch_root(cur_block_height, blocks_per_epoch) {
                    return;
                }

                let Ok(auth_root) = leaf.block_header().auth_root() else {
                    tracing::error!("Failed to get auth root for light client state");
                    return;
                };
                let auth_root = FixedBytes::from(auth_root.unwrap_or([0u8; 32]));

                let option_state_epoch = option_epoch_from_block_number::<SeqTypes>(
                    leaf.with_epoch,
                    cur_block_height,
                    blocks_per_epoch,
                );

                if self.voting_stake_table_epoch != option_state_epoch {
                    let Ok(membership) = consensus
                        .membership_coordinator
                        .stake_table_for_epoch(option_state_epoch)
                        .await
                    else {
                        tracing::error!(
                            "Fail to get membership for epoch: {:?}",
                            option_state_epoch
                        );
                        return;
                    };
                    match membership
                        .stake_table()
                        .await
                        .commitment(self.stake_table_capacity)
                    {
                        Ok(stake_table_state) => {
                            self.voting_stake_table_epoch = option_state_epoch;
                            self.voting_stake_table = stake_table_state;
                        },
                        Err(err) => {
                            tracing::error!("Failed to compute stake table commitment: {:?}", err);
                            return;
                        },
                    }
                }

                if let Some(client) = &self.relay_server_client {
                    let Ok(request_body) = self
                        .get_request_body(&state, &self.voting_stake_table, auth_root)
                        .await
                    else {
                        tracing::error!("Failed to sign new state");
                        return;
                    };

                    if let Err(error) = client
                        .post::<()>("api/state")
                        .body_binary(&request_body)
                        .unwrap()
                        .send()
                        .await
                    {
                        tracing::error!("Error posting signature to the relay server: {:?}", error);
                    }

                    if !leaf.with_epoch {
                        // Before epoch upgrade, we need to sign the state for the legacy light client
                        let Ok(legacy_signature) = self.legacy_sign_new_state(&state).await else {
                            tracing::error!("Failed to sign new state for legacy light client");
                            return;
                        };
                        let legacy_request_body = LCV2StateSignatureRequestBody {
                            key: self.ver_key.clone(),
                            state,
                            next_stake: StakeTableState::default(),
                            signature: legacy_signature,
                        };
                        if let Err(error) = client
                            .post::<()>("api/legacy-state")
                            .body_binary(&legacy_request_body)
                            .unwrap()
                            .send()
                            .await
                        {
                            tracing::error!(
                                "Error posting signature for legacy light client to the relay \
                                 server: {:?}",
                                error
                            );
                        }
                    }
                }
            },
            Err(err) => {
                tracing::error!("Error generating light client state: {:?}", err)
            },
        }
    }

    /// Return a signature of a light client state at given height.
    pub async fn get_state_signature(&self, height: u64) -> Option<LCV3StateSignatureRequestBody> {
        let pool_guard = self.signatures.read().await;
        pool_guard.get_signature(height)
    }

    /// Sign the light client state at given height and store it.
    async fn get_request_body(
        &self,
        state: &LightClientState,
        next_stake_table: &StakeTableState,
        auth_root: FixedBytes<32>,
    ) -> Result<LCV3StateSignatureRequestBody, SignatureError> {
        let signed_state_digest = derive_signed_state_digest(state, &next_stake_table, &auth_root);
        let signature = <SchnorrPubKey as LCV3StateSignatureKey>::sign_state(
            &self.sign_key,
            signed_state_digest,
        )?;
        let v2signature = <SchnorrPubKey as LCV2StateSignatureKey>::sign_state(
            &self.sign_key,
            state,
            next_stake_table,
        )?;
        let request_body = LCV3StateSignatureRequestBody {
            key: self.ver_key.clone(),
            state: *state,
            next_stake: next_stake_table.clone(),
            signature,
            v2_signature: v2signature.clone(),
            auth_root,
        };
        let mut pool_guard = self.signatures.write().await;
        pool_guard.push(state.block_height, request_body.clone());
        tracing::debug!(
            "New signature added for block height {}",
            state.block_height
        );
        Ok(request_body)
    }

    async fn legacy_sign_new_state(
        &self,
        state: &LightClientState,
    ) -> Result<StateSignature, SignatureError> {
        <SchnorrPubKey as LCV1StateSignatureKey>::sign_state(&self.sign_key, state)
    }
}

/// A rolling in-memory storage for the most recent light client state signatures.
#[derive(Debug, Default)]
pub struct StateSignatureMemStorage {
    pool: HashMap<u64, LCV3StateSignatureRequestBody>,
    deque: VecDeque<u64>,
}

impl StateSignatureMemStorage {
    pub fn push(&mut self, height: u64, signature: LCV3StateSignatureRequestBody) {
        self.pool.insert(height, signature);
        self.deque.push_back(height);
        if self.pool.len() > SIGNATURE_STORAGE_CAPACITY {
            self.pool.remove(&self.deque.pop_front().unwrap());
        }
    }

    pub fn get_signature(&self, height: u64) -> Option<LCV3StateSignatureRequestBody> {
        self.pool.get(&height).cloned()
    }
}
