//! Helpers and test mocks for Light Client logic

use alloy::primitives::U256;
use ark_ff::PrimeField;
use hotshot_types::light_client::{GenericLightClientState, GenericStakeTableState};

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
            blockCommRoot: U256::from(0),
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
            view_number: v.viewNum as usize,
            block_height: v.blockHeight as usize,
            block_comm_root: u256_to_field(v.blockCommRoot),
        }
    }
}

impl<F: PrimeField> From<GenericLightClientState<F>> for LightClientStateSol {
    fn from(v: GenericLightClientState<F>) -> Self {
        Self {
            viewNum: v.view_number as u64,
            blockHeight: v.block_height as u64,
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
