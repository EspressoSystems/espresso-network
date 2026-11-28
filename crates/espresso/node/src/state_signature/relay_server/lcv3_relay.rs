use std::collections::{BTreeSet, HashMap, hash_map::Entry};

use alloy::primitives::U256;
use axum::http::StatusCode;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    light_client::{
        LCV3StateSignatureRequestBody, LCV3StateSignaturesBundle, LightClientState, StateVerKey,
    },
    traits::signature_key::LCV3StateSignatureKey,
};

use super::{RelayError, stake_table_tracker::StakeTableInfo};

pub trait LCV3StateRelayServerDataSource {
    /// Get the latest available signatures bundle.
    /// # Errors
    /// Errors if there's no available signatures bundle.
    fn get_latest_signature_bundle(&self) -> Result<LCV3StateSignaturesBundle, RelayError>;

    /// Post a signature to the relay server
    /// # Errors
    /// Errors if the signature is invalid, already posted, or no longer needed.
    fn post_signature(
        &mut self,
        req: LCV3StateSignatureRequestBody,
        stake_table: &StakeTableInfo,
    ) -> Result<(), RelayError>;
}

/// Server state that tracks the light client V3 state and signatures
#[derive(Default)]
pub struct LCV3StateRelayServerState {
    /// Bundles for light client V3
    bundles: HashMap<u64, HashMap<LightClientState, LCV3StateSignaturesBundle>>,

    /// The latest state signatures bundle for LCV3 light client
    latest_available_bundle: Option<LCV3StateSignaturesBundle>,
    /// The block height of the latest available LCV3 state signature bundle
    latest_block_height: Option<u64>,

    /// A ordered queue of block heights for V3 light client state, used for garbage collection.
    gc_queue: BTreeSet<u64>,
}

impl LCV3StateRelayServerDataSource for LCV3StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV3StateSignaturesBundle, RelayError> {
        self.latest_available_bundle
            .clone()
            .ok_or(RelayError::catch_all(
                StatusCode::NOT_FOUND,
                "The light client V3 state signatures are not ready.".to_owned(),
            ))
    }

    fn post_signature(
        &mut self,
        req: LCV3StateSignatureRequestBody,
        stake_table: &StakeTableInfo,
    ) -> Result<(), RelayError> {
        let block_height = req.state.block_height;
        if block_height <= self.latest_block_height.unwrap_or(0) {
            // This signature is no longer needed
            return Ok(());
        }
        let Some(weight) = stake_table.known_nodes.get(&req.key) else {
            tracing::warn!("Received LCV3 signature from unknown node: {req}");
            return Err(RelayError::catch_all(
                StatusCode::UNAUTHORIZED,
                "LCV3 signature posted by nodes not on the stake table".to_owned(),
            ));
        };

        // sanity check the signature validity first before adding in
        let signed_state_digest =
            derive_signed_state_digest(&req.state, &req.next_stake, &req.auth_root);
        if !<StateVerKey as LCV3StateSignatureKey>::verify_state_sig(
            &req.key,
            &req.signature,
            signed_state_digest,
        ) {
            tracing::warn!("Couldn't verify the received LCV3 signature: {req}");
            return Err(RelayError::catch_all(
                StatusCode::BAD_REQUEST,
                "The posted LCV3 signature is not valid.".to_owned(),
            ));
        }

        let bundles_at_height = self.bundles.entry(block_height).or_default();
        self.gc_queue.insert(block_height);

        let bundle = bundles_at_height
            .entry(req.state)
            .or_insert(LCV3StateSignaturesBundle {
                state: req.state,
                next_stake: req.next_stake,
                auth_root: req.auth_root,
                signatures: Default::default(),
                accumulated_weight: U256::from(0),
            });
        tracing::debug!(
            "Accepting new LCV3 signature for block height {} from {}.",
            block_height,
            req.key
        );
        match bundle.signatures.entry(req.key) {
            Entry::Occupied(_) => {
                // A signature is already posted for this key with this state
                return Err(RelayError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "A LCV3 signature of this light client state is already posted at this block \
                     height for this key."
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
                "Light client V3 state signature bundle at block height {} is ready to serve.",
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

impl LCV3StateRelayServerState {
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
}
