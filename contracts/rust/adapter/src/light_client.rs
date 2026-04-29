//! Helpers and test mocks for Light Client logic

use std::collections::{HashMap, HashSet};

use alloy::{
    primitives::{FixedBytes, U256},
    sol_types::SolValue,
};
use ark_ff::PrimeField;
use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    light_client::{
        CircuitField, GenericLightClientState, GenericStakeTableState, LightClientState,
        StakeTableState,
    },
    message::UpgradeLock,
    simple_certificate::LightClientStateUpdateCertificateV2,
    simple_vote::HasEpoch,
    traits::{
        node_implementation::NodeType,
        signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
    },
};
use hotshot_utils::anytrace::*;
use rand::Rng;

use crate::{
    field_to_u256,
    sol_types::{LightClient, LightClientStateSol, StakeTableStateSol},
    u256_to_field,
};

impl LightClientStateSol {
    /// Return a dummy new genesis that will pass constructor/initializer sanity checks
    /// in the contract.
    ///
    /// # Warning
    /// NEVER use this for production, this is test only.
    pub fn dummy_genesis() -> Self {
        Self {
            viewNum: 0,
            blockHeight: 0,
            blockCommRoot: U256::from(42),
        }
    }

    /// Return a random value
    pub fn rand<R: Rng>(rng: &mut R) -> Self {
        Self {
            viewNum: rng.r#gen::<u64>(),
            blockHeight: rng.r#gen::<u64>(),
            blockCommRoot: U256::from_limbs(rng.r#gen::<[u64; 4]>()),
        }
    }
}

impl From<LightClient::finalizedStateReturn> for LightClientStateSol {
    fn from(v: LightClient::finalizedStateReturn) -> Self {
        let tuple: (u64, u64, U256) = v.into();
        tuple.into()
    }
}

impl<F: PrimeField> From<LightClientStateSol> for GenericLightClientState<F> {
    fn from(v: LightClientStateSol) -> Self {
        Self {
            view_number: v.viewNum,
            block_height: v.blockHeight,
            block_comm_root: u256_to_field(v.blockCommRoot),
        }
    }
}

impl<F: PrimeField> From<GenericLightClientState<F>> for LightClientStateSol {
    fn from(v: GenericLightClientState<F>) -> Self {
        Self {
            viewNum: v.view_number,
            blockHeight: v.block_height,
            blockCommRoot: field_to_u256(v.block_comm_root),
        }
    }
}

impl StakeTableStateSol {
    /// Return a dummy new genesis stake state that will pass constructor/initializer sanity checks
    /// in the contract.
    ///
    /// # Warning
    /// NEVER use this for production, this is test only.
    pub fn dummy_genesis() -> Self {
        Self {
            threshold: U256::from(1),
            blsKeyComm: U256::from(123),
            schnorrKeyComm: U256::from(123),
            amountComm: U256::from(20),
        }
    }

    /// Returns a random value
    pub fn rand<R: Rng>(rng: &mut R) -> Self {
        Self {
            threshold: U256::from_limbs(rng.r#gen::<[u64; 4]>()),
            blsKeyComm: U256::from_limbs(rng.r#gen::<[u64; 4]>()),
            schnorrKeyComm: U256::from_limbs(rng.r#gen::<[u64; 4]>()),
            amountComm: U256::from_limbs(rng.r#gen::<[u64; 4]>()),
        }
    }
}

impl From<LightClient::genesisStakeTableStateReturn> for StakeTableStateSol {
    fn from(v: LightClient::genesisStakeTableStateReturn) -> Self {
        let tuple: (U256, U256, U256, U256) = v.into();
        tuple.into()
    }
}

impl<F: PrimeField> From<StakeTableStateSol> for GenericStakeTableState<F> {
    fn from(s: StakeTableStateSol) -> Self {
        Self {
            threshold: u256_to_field(s.threshold),
            bls_key_comm: u256_to_field(s.blsKeyComm),
            schnorr_key_comm: u256_to_field(s.schnorrKeyComm),
            amount_comm: u256_to_field(s.amountComm),
        }
    }
}

impl<F: PrimeField> From<GenericStakeTableState<F>> for StakeTableStateSol {
    fn from(v: GenericStakeTableState<F>) -> Self {
        Self {
            blsKeyComm: field_to_u256(v.bls_key_comm),
            schnorrKeyComm: field_to_u256(v.schnorr_key_comm),
            amountComm: field_to_u256(v.amount_comm),
            threshold: field_to_u256(v.threshold),
        }
    }
}

/// Derive the signed state digest used for LCV3 light-client signatures:
/// `keccak256(abi.encodePacked(abi.encode(state) || abi.encode(stake) || abi.encode(auth_root)))`,
/// converted to a `CircuitField`.
pub fn derive_signed_state_digest(
    lc_state: &LightClientState,
    next_stake_state: &StakeTableState,
    auth_root: &FixedBytes<32>,
) -> CircuitField {
    let lc_state_sol: LightClientStateSol = (*lc_state).into();
    let stake_st_sol: StakeTableStateSol = (*next_stake_state).into();

    let res = alloy::primitives::keccak256(
        (
            lc_state_sol.abi_encode(),
            stake_st_sol.abi_encode(),
            auth_root.abi_encode(),
        )
            .abi_encode_packed(),
    );
    CircuitField::from_be_bytes_mod_order(res.as_ref())
}

/// Validates a light client state update certificate:
/// - every signer is in the voting stake table for the cert's epoch
/// - each signature is valid (LCV2 always; LCV3 once post-DrbAndHeaderUpgrade)
/// - the accumulated stake of signers meets the success threshold
pub async fn validate_light_client_state_update_certificate<TYPES: NodeType>(
    state_cert: &LightClientStateUpdateCertificateV2<TYPES>,
    membership_coordinator: &EpochMembershipCoordinator<TYPES>,
    upgrade_lock: &UpgradeLock<TYPES>,
) -> Result<()> {
    tracing::debug!("Validating light client state update certificate");

    let epoch_membership = membership_coordinator
        .membership_for_epoch(state_cert.epoch())
        .await?;

    let membership_stake_table = epoch_membership.stake_table().await;
    let membership_success_threshold = epoch_membership.success_threshold().await;

    let mut state_key_map = HashMap::new();
    membership_stake_table.into_iter().for_each(|config| {
        state_key_map.insert(
            config.state_ver_key.clone(),
            config.stake_table_entry.stake(),
        );
    });

    let mut accumulated_stake = U256::from(0);
    let mut seen_keys = HashSet::new();
    let signed_state_digest = derive_signed_state_digest(
        &state_cert.light_client_state,
        &state_cert.next_stake_table_state,
        &state_cert.auth_root,
    );
    for (key, sig, sig_v2) in state_cert.signatures.iter() {
        if !seen_keys.insert(key.clone()) {
            bail!("Duplicate signature for key: {key:?}");
        }
        if let Some(stake) = state_key_map.get(key) {
            accumulated_stake += *stake;
            #[allow(clippy::collapsible_else_if)]
            // We only perform the second signature check prior to the DrbAndHeaderUpgrade
            if !upgrade_lock
                .proposal2_version(ViewNumber::new(state_cert.light_client_state.view_number))
            {
                if !<TYPES::StateSignatureKey as LCV2StateSignatureKey>::verify_state_sig(
                    key,
                    sig_v2,
                    &state_cert.light_client_state,
                    &state_cert.next_stake_table_state,
                ) {
                    bail!("Invalid light client state update certificate signature");
                }
            } else {
                if !<TYPES::StateSignatureKey as LCV3StateSignatureKey>::verify_state_sig(
                    key,
                    sig,
                    signed_state_digest,
                ) || !<TYPES::StateSignatureKey as LCV2StateSignatureKey>::verify_state_sig(
                    key,
                    sig_v2,
                    &state_cert.light_client_state,
                    &state_cert.next_stake_table_state,
                ) {
                    bail!("Invalid light client state update certificate signature");
                }
            }
        } else {
            bail!("Invalid light client state update certificate signature");
        }
    }
    if accumulated_stake < membership_success_threshold {
        bail!("Light client state update certificate does not meet the success threshold");
    }

    Ok(())
}
