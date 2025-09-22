use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap},
    sync::Arc,
};

use alloy::primitives::U256;
use hotshot_types::{
    light_client::{
        LCV2StateSignatureRequestBody, LCV2StateSignaturesBundle, LightClientState, StateVerKey,
    },
    traits::signature_key::LCV2StateSignatureKey,
};
use tide_disco::{error::ServerError, Error, StatusCode};

use super::stake_table_tracker::StakeTableTracker;

#[async_trait::async_trait]
pub trait LCV2StateRelayServerDataSource {
    /// Get the latest available signatures bundle.
    /// # Errors
    /// Errors if there's no available signatures bundle.
    fn get_latest_signature_bundle(&self) -> Result<LCV2StateSignaturesBundle, ServerError>;

    /// Post a signature to the relay server
    /// # Errors
    /// Errors if the signature is invalid, already posted, or no longer needed.
    async fn post_signature(
        &mut self,
        req: LCV2StateSignatureRequestBody,
    ) -> Result<(), ServerError>;
}

/// Server state that tracks the light client V2 state and signatures
pub struct LCV2StateRelayServerState {
    /// Bundles for light client V2
    bundles: HashMap<u64, HashMap<LightClientState, LCV2StateSignaturesBundle>>,

    /// The latest state signatures bundle for LCV2 light client
    latest_available_bundle: Option<LCV2StateSignaturesBundle>,
    /// The block height of the latest available LCV2 state signature bundle
    latest_block_height: Option<u64>,

    /// A ordered queue of block heights for V2 light client state, used for garbage collection.
    gc_queue: BTreeSet<u64>,

    /// Stake table tracker
    stake_table_tracker: Arc<StakeTableTracker>,
}

#[async_trait::async_trait]
impl LCV2StateRelayServerDataSource for LCV2StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV2StateSignaturesBundle, ServerError> {
        self.latest_available_bundle
            .clone()
            .ok_or(ServerError::catch_all(
                StatusCode::NOT_FOUND,
                "The light client V2 state signatures are not ready.".to_owned(),
            ))
    }

    async fn post_signature(
        &mut self,
        req: LCV2StateSignatureRequestBody,
    ) -> Result<(), ServerError> {
        let block_height = req.state.block_height;
        if block_height <= self.latest_block_height.unwrap_or(0) {
            // This signature is no longer needed
            return Ok(());
        }
        let stake_table = self
            .stake_table_tracker
            .stake_table_info_for_block(block_height)
            .await
            .map_err(|e| {
                ServerError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;
        let Some(weight) = stake_table.known_nodes.get(&req.key) else {
            tracing::warn!("Received LCV2 signature from unknown node: {req}");
            return Err(ServerError::catch_all(
                StatusCode::UNAUTHORIZED,
                "LCV2 signature posted by nodes not on the stake table".to_owned(),
            ));
        };

        // sanity check the signature validity first before adding in
        if !<StateVerKey as LCV2StateSignatureKey>::verify_state_sig(
            &req.key,
            &req.signature,
            &req.state,
            &req.next_stake,
        ) {
            tracing::warn!("Couldn't verify the received LCV2 signature: {req}");
            return Err(ServerError::catch_all(
                StatusCode::BAD_REQUEST,
                "The posted LCV2 signature is not valid.".to_owned(),
            ));
        }

        let bundles_at_height = self.bundles.entry(block_height).or_default();
        self.gc_queue.insert(block_height);

        let bundle = bundles_at_height
            .entry(req.state)
            .or_insert(LCV2StateSignaturesBundle {
                state: req.state,
                next_stake: req.next_stake,
                signatures: Default::default(),
                accumulated_weight: U256::from(0),
            });
        tracing::debug!(
            "Accepting new LCV2 signature for block height {} from {}.",
            block_height,
            req.key
        );
        match bundle.signatures.entry(req.key) {
            Entry::Occupied(_) => {
                // A signature is already posted for this key with this state
                return Err(ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "A LCV2 signature of this light client state is already posted at this \
                     block height for this key."
                        .to_owned(),
                ));
            },
            Entry::Vacant(entry) => {
                entry.insert(req.signature);
                bundle.accumulated_weight += *weight;
            },
        }

        if bundle.accumulated_weight >= stake_table.threshold {
            tracing::info!(
                "Light client V2 state signature bundle at block height {} is ready to serve.",
                block_height
            );
            self.latest_block_height = Some(block_height);
            self.latest_available_bundle = Some(bundle.clone());

            // garbage collect
            self.prune(block_height);
        }

        Ok(())
    }
}

impl LCV2StateRelayServerState {
    /// Centralizing all garbage-collection logic, won't panic, won't error, simply do nothing if nothing to prune.
    /// `until_height` is inclusive, meaning that would also be pruned.
    pub fn prune(&mut self, until_height: u64) {
        while let Some(&height) = self.gc_queue.first() {
            if height > until_height {
                return;
            }
            self.bundles.remove(&height);
            self.gc_queue.pop_first();
            tracing::debug!(%height, "garbage collected for ");
        }
    }

    pub fn new(stake_table_tracker: Arc<StakeTableTracker>) -> Self {
        Self {
            bundles: HashMap::new(),
            latest_available_bundle: None,
            latest_block_height: None,
            gc_queue: BTreeSet::new(),
            stake_table_tracker,
        }
    }
}
