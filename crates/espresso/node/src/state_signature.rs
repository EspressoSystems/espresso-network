//! Utilities for generating and storing the most recent light client state signatures.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use alloy::primitives::FixedBytes;
use async_lock::RwLock;
use espresso_types::{Header, traits::SequencerPersistence};
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::{
        LCV2StateSignatureRequestBody, LCV3StateSignatureRequestBody, LightClientState,
        StakeTableState, StateSignKey, StateVerKey,
    },
    new_protocol::CoordinatorEvent,
    signature_key::SchnorrPubKey,
    stake_table::HSStakeTable,
    traits::{
        block_contents::BlockHeader,
        election::Membership,
        signature_key::{
            LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey,
            StakeTableEntryType,
        },
    },
    utils::{is_ge_epoch_root, option_epoch_from_block_number},
};
use http_client::{Client, Url, error::ClientErr};
use jf_signature::SignatureError;
use vbs::version::StaticVersionType;

use crate::{SeqTypes, consensus_handle::ConsensusHandle};

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

    /// The most recent light client state signatures, shared with readers that must not wait on
    /// the signer.
    pub(crate) signatures: Arc<RwLock<StateSignatureMemStorage>>,

    /// Commitment for current fixed stake table
    voting_stake_table_state: StakeTableState,

    /// epoch for the current stake table state
    voting_stake_table_epoch: Option<EpochNumber>,

    /// Capacity of the stake table
    stake_table_capacity: usize,

    /// Indicate whether the signer is in the voting stake table and should sign the state
    should_vote: bool,

    /// The state relay server url
    relay_server_client: Option<Client<ClientErr, ApiVer>>,
}

impl<ApiVer: StaticVersionType> StateSigner<ApiVer> {
    pub fn new(
        sign_key: StateSignKey,
        ver_key: StateVerKey,
        voting_stake_table_state: StakeTableState,
        voting_stake_table_epoch: Option<EpochNumber>,
        stake_table_capacity: usize,
        should_vote: bool,
    ) -> Self {
        Self {
            sign_key,
            ver_key,
            voting_stake_table_state,
            voting_stake_table_epoch,
            stake_table_capacity,
            signatures: Default::default(),
            should_vote,
            relay_server_client: Default::default(),
        }
    }

    /// Connect to the given state relay server to send signed HotShot states to.
    pub fn with_relay_server(mut self, url: Url) -> Self {
        self.relay_server_client = Some(Client::new(url));
        self
    }

    pub(super) async fn handle_decide<P: SequencerPersistence>(
        &mut self,
        leaf: &DecidedLeaf,
        consensus_handle: &ConsensusHandle<SeqTypes, Arc<P>>,
    ) {
        match leaf.header.get_light_client_state(leaf.view) {
            Ok(state) => {
                tracing::debug!(
                    "New leaves decided. Latest block height: {}",
                    leaf.header.block_number()
                );

                let cur_block_height = state.block_height;
                let blocks_per_epoch = *consensus_handle.epoch_height();

                let option_state_epoch = option_epoch_from_block_number(
                    leaf.with_epoch,
                    cur_block_height,
                    blocks_per_epoch,
                );

                if self.voting_stake_table_epoch != option_state_epoch {
                    let Some(stake_table) = voting_stake_table(
                        &consensus_handle.membership_coordinator(),
                        option_state_epoch,
                    ) else {
                        return;
                    };
                    match stake_table.commitment(self.stake_table_capacity) {
                        Ok(stake_table_state) => {
                            self.should_vote = should_vote(&stake_table, &self.ver_key);
                            self.voting_stake_table_epoch = option_state_epoch;
                            self.voting_stake_table_state = stake_table_state;
                        },
                        Err(err) => {
                            tracing::error!("Failed to compute stake table commitment: {:?}", err);
                            return;
                        },
                    }
                }

                if !self.should_vote {
                    tracing::debug!(
                        "Not signing the state at block height {} since not in the voting stake \
                         table",
                        state.block_height
                    );
                    return;
                }

                // The last few state updates are handled in the consensus, we do not sign them.
                if leaf.with_epoch & is_ge_epoch_root(cur_block_height, blocks_per_epoch) {
                    tracing::debug!("Skipping epoch transition block {cur_block_height}");
                    return;
                }

                let Ok(auth_root) = leaf.header.auth_root() else {
                    tracing::error!("Failed to get auth root for light client state");
                    return;
                };

                let Ok(request_body) = self
                    .get_request_body(&state, &self.voting_stake_table_state, auth_root)
                    .await
                else {
                    tracing::error!("Failed to sign new state");
                    return;
                };

                if let Some(client) = &self.relay_server_client {
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
                        let Ok(legacy_signature) =
                            <SchnorrPubKey as LCV1StateSignatureKey>::sign_state(
                                &self.sign_key,
                                &state,
                            )
                        else {
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

    /// Sign the light client state at given height and store it.
    async fn get_request_body(
        &self,
        state: &LightClientState,
        next_stake_table: &StakeTableState,
        auth_root: FixedBytes<32>,
    ) -> Result<LCV3StateSignatureRequestBody, SignatureError> {
        let signed_state_digest = derive_signed_state_digest(state, next_stake_table, &auth_root);
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
            next_stake: *next_stake_table,
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
}

/// The parts of a decide's newest leaf that the state signer needs, without the block payload
/// a leader's leaf still carries.
#[derive(Clone, Debug)]
pub(crate) struct DecidedLeaf {
    header: Header,
    view: ViewNumber,
    with_epoch: bool,
}

impl DecidedLeaf {
    pub(crate) fn from_event(event: &CoordinatorEvent<SeqTypes>) -> Option<DecidedLeaf> {
        let leaf = match event {
            CoordinatorEvent::NewDecide { leaf_infos, .. } => &leaf_infos.first()?.leaf,
            _ => return None,
        };
        Some(DecidedLeaf {
            header: leaf.block_header().clone(),
            view: leaf.view_number(),
            with_epoch: leaf.with_epoch,
        })
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

/// The stake table that signs the states of `epoch`, or `None` if it is not
/// available. States from before epochs were enabled have no epoch, and the
/// genesis stake table signs them.
fn voting_stake_table(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch: Option<EpochNumber>,
) -> Option<HSStakeTable<SeqTypes>> {
    let Some(epoch) = epoch else {
        let genesis = coordinator.membership().genesis_stake_tables();
        return Some(HSStakeTable::from_iter(genesis.stake_table()));
    };
    let Ok(membership) = coordinator.stake_table_for_epoch(epoch) else {
        tracing::error!("Failed to get membership for epoch: {epoch}");
        return None;
    };
    Some(HSStakeTable::from_iter(membership.stake_table()))
}

pub(crate) fn should_vote(stake_table: &HSStakeTable<SeqTypes>, ver_key: &SchnorrPubKey) -> bool {
    stake_table
        .0
        .iter()
        .any(|peer| &peer.state_ver_key == ver_key && !peer.stake_table_entry.stake().is_zero())
}

#[cfg(test)]
mod test {
    use espresso_types::{TEST_UPGRADE, traits::PersistenceOptions};
    use futures::StreamExt;
    use hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY;
    use test_utils::reserve_tcp_port;

    use super::*;
    use crate::{
        SequencerApiVersion,
        api::{
            Options,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
        },
        testing::{TestConfigBuilder, decided_leaves},
    };

    // A leaf decided before epochs were enabled has no epoch. Its state is
    // signed with the genesis stake table as the next stake table.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_decide_signs_leaf_without_epoch_with_genesis_stake_table() {
        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let config = TestNetworkConfigBuilder::default()
            .api_config(Options::with_port(port))
            .network_config(TestConfigBuilder::default().build())
            .build();
        let network = TestNetwork::new(config, TEST_UPGRADE).await;

        let (leaf, height) = decided_leaf_without_epoch(&network).await;
        let validator = network.server.validator_config();
        let mut signer = StateSigner::<SequencerApiVersion>::new(
            validator.state_private_key.clone(),
            validator.state_public_key.clone(),
            StakeTableState::default(),
            // Any epoch but the leaf's makes the signer pick its stake table again.
            Some(EpochNumber::genesis()),
            DEFAULT_STAKE_TABLE_CAPACITY,
            false,
        );
        signer
            .handle_decide(&leaf, network.server.consensus_handle().as_ref())
            .await;

        // Every genesis member has stake, so the genesis stake table is the configured one.
        let genesis = HSStakeTable::from_iter(network.cfg.known_nodes_with_stake())
            .commitment(DEFAULT_STAKE_TABLE_CAPACITY)
            .expect("genesis stake table commitment");
        let signature = signer
            .signatures
            .read()
            .await
            .get_signature(height)
            .unwrap_or_else(|| panic!("no signature for height {height}"));
        assert_eq!(signature.next_stake, genesis, "next stake table");
    }

    /// The newest leaf of the next decide of `network`, with its epoch
    /// removed, and its height.
    async fn decided_leaf_without_epoch<P: PersistenceOptions, const N: usize>(
        network: &TestNetwork<P, N>,
    ) -> (DecidedLeaf, u64) {
        let mut events = network.server.event_stream();
        let mut event = loop {
            let event = events.next().await.expect("event stream ended");
            if decided_leaves(&event).is_some() {
                break event;
            }
        };
        let CoordinatorEvent::NewDecide { leaf_infos, .. } = &mut event else {
            unreachable!("a decide event");
        };
        leaf_infos[0].leaf.with_epoch = false;
        let height = leaf_infos[0].leaf.height();
        let leaf = DecidedLeaf::from_event(&event).expect("a decided leaf");
        (leaf, height)
    }
}
