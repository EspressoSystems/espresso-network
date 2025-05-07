// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs related to the stake table

use alloy::primitives::U256;
use ark_ff::PrimeField;
use jf_crhf::CRHF;
use jf_rescue::crhf::VariableLengthRescueCRHF;
use serde::{Deserialize, Serialize};

use crate::{
    light_client::{CircuitField, StakeTableState, ToFieldsLightClientCompat},
    traits::signature_key::{SignatureKey, StakeTableEntryType},
    NodeType, PeerConfig,
};

/// Stake table entry
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct StakeTableEntry<K: SignatureKey> {
    /// The public key
    pub stake_key: K,
    /// The associated stake amount
    pub stake_amount: U256,
}

impl<K: SignatureKey> StakeTableEntryType<K> for StakeTableEntry<K> {
    /// Get the stake amount
    fn stake(&self) -> U256 {
        self.stake_amount
    }

    /// Get the public key
    fn public_key(&self) -> K {
        self.stake_key.clone()
    }
}

impl<K: SignatureKey> StakeTableEntry<K> {
    /// Get the public key
    pub fn key(&self) -> &K {
        &self.stake_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FullStakeTable<TYPES: NodeType>(pub Vec<PeerConfig<TYPES>>);

impl<TYPES: NodeType> From<Vec<PeerConfig<TYPES>>> for FullStakeTable<TYPES> {
    fn from(peers: Vec<PeerConfig<TYPES>>) -> Self {
        Self(peers)
    }
}

impl<TYPES: NodeType> std::ops::Deref for FullStakeTable<TYPES> {
    type Target = Vec<PeerConfig<TYPES>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TYPES: NodeType> std::ops::DerefMut for FullStakeTable<TYPES> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[inline]
/// A helper function to compute the quorum threshold given a total amount of stake.
pub fn one_honest_threshold(total_stake: U256) -> U256 {
    total_stake / U256::from(3) + U256::from(1)
}

#[inline]
fn u256_to_field(amount: U256) -> CircuitField {
    let amount_bytes: [u8; 32] = amount.to_le_bytes();
    CircuitField::from_le_bytes_mod_order(&amount_bytes)
}

impl<TYPES: NodeType> FullStakeTable<TYPES> {
    pub fn commitment(&self, stake_table_capacity: usize) -> anyhow::Result<StakeTableState> {
        if stake_table_capacity < self.0.len() {
            return Err(anyhow::anyhow!(
                "Stake table over capacity: {} < {}",
                stake_table_capacity,
                self.0.len(),
            ));
        }
        let padding_len = stake_table_capacity - self.0.len();
        let mut bls_preimage = vec![];
        let mut schnorr_preimage = vec![];
        let mut amount_preimage = vec![];
        let mut total_stake = U256::from(0);
        for peer in &self.0 {
            bls_preimage.extend(peer.stake_table_entry.public_key().to_fields());
            schnorr_preimage.extend(peer.state_ver_key.to_fields());
            amount_preimage.push(u256_to_field(peer.stake_table_entry.stake()));
            total_stake += peer.stake_table_entry.stake();
        }
        bls_preimage.resize(
            <TYPES::SignatureKey as ToFieldsLightClientCompat>::SIZE * stake_table_capacity,
            CircuitField::default(),
        );
        // Nasty tech debt
        schnorr_preimage.extend(
            std::iter::repeat_n(TYPES::StateSignatureKey::default().to_fields(), padding_len)
                .flatten(),
        );
        amount_preimage.resize(stake_table_capacity, CircuitField::default());
        let threshold = u256_to_field(one_honest_threshold(total_stake));
        Ok(StakeTableState {
            bls_key_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(bls_preimage)
                .unwrap()[0],
            schnorr_key_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(
                schnorr_preimage,
            )
            .unwrap()[0],
            amount_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(amount_preimage)
                .unwrap()[0],
            threshold,
        })
    }
}
