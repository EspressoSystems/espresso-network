// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! A vector based stake table implementation. The commitment is the rescue hash of the list of (key, amount) pairs;

use alloy::primitives::{U256, U512};
use ark_std::{collections::HashMap, hash::Hash, rand::SeedableRng};
use digest::crypto_common::rand_core::CryptoRngCore;
use hotshot_types::{
    light_client::GenericStakeTableState,
    traits::stake_table::{SnapshotVersion, StakeTableError, StakeTableScheme},
};
use jf_crhf::CRHF;
use jf_rescue::{crhf::VariableLengthRescueCRHF, RescueParameter};
use serde::{Deserialize, Serialize};

use crate::{
    config::STAKE_TABLE_CAPACITY,
    utils::{one_honest_threshold, u256_to_field, ToFields},
};

pub mod config;

/// a snapshot of the stake table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StakeTableSnapshot<K1, K2> {
    /// bls keys
    pub bls_keys: Vec<K1>,
    /// schnorr
    pub schnorr_keys: Vec<K2>,
    /// amount of stake
    pub stake_amount: Vec<U256>,
}

impl<K1, K2> Default for StakeTableSnapshot<K1, K2> {
    fn default() -> Self {
        Self {
            bls_keys: vec![],
            schnorr_keys: vec![],
            stake_amount: vec![],
        }
    }
}

/// Locally maintained stake table, generic over public key type `K`.
/// Whose commitment is a rescue hash of all key-value pairs over field `F`.
/// NOTE: the commitment is only available for the finalized versions, and is
/// computed only once when it's finalized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeTable<K1, K2, F>
where
    K1: Eq + Hash + Clone + ToFields<F>,
    K2: Eq + Hash + Clone + Default + ToFields<F>,
    F: RescueParameter,
{
    /// upper bound on table size
    capacity: usize,
    /// The most up-to-date stake table, where the incoming transactions shall be performed on.
    head: StakeTableSnapshot<K1, K2>,
    /// The snapshot of stake table at the beginning of the current epoch
    epoch_start: StakeTableSnapshot<K1, K2>,
    /// The stake table used for leader election.
    last_epoch_start: StakeTableSnapshot<K1, K2>,

    /// Total stakes in the most update-to-date stake table
    head_total_stake: U256,
    /// Total stakes in the snapshot version `EpochStart`
    epoch_start_total_stake: U256,
    /// Total stakes in the snapshot version `LastEpochStart`
    last_epoch_start_total_stake: U256,

    /// Commitment of the stake table snapshot version `EpochStart`
    /// We only support committing the finalized versions.
    /// Commitment for a finalized version is a triple where
    ///  - First item is the rescue hash of the bls keys
    ///  - Second item is the rescue hash of the Schnorr keys
    ///  - Third item is the rescue hash of all the stake amounts
    epoch_start_comm: (F, F, F),

    /// Commitment of the stake table snapshot version `LastEpochStart`
    last_epoch_start_comm: (F, F, F),

    /// The mapping from public keys to their location in the Merkle tree.
    #[serde(skip)]
    bls_mapping: HashMap<K1, usize>,
}

impl<K1, K2, F> StakeTableScheme for StakeTable<K1, K2, F>
where
    K1: Eq + Hash + Clone + ToFields<F>,
    K2: Eq + Hash + Clone + Default + ToFields<F>,
    F: RescueParameter,
{
    /// The stake table is indexed by BLS key
    type Key = K1;
    /// The auxiliary information is the associated Schnorr key
    type Aux = K2;
    type Amount = U256;
    type Commitment = (F, F, F);
    type LookupProof = ();
    // TODO(Chengyu): Can we make it references?
    type IntoIter = <Vec<(K1, U256, K2)> as ark_std::iter::IntoIterator>::IntoIter;

    fn register(
        &mut self,
        new_key: Self::Key,
        amount: Self::Amount,
        aux: Self::Aux,
    ) -> Result<(), StakeTableError> {
        if self.bls_mapping.contains_key(&new_key) {
            Err(StakeTableError::ExistingKey)
        } else {
            let pos = self.bls_mapping.len();
            self.head.bls_keys.push(new_key.clone());
            self.head.schnorr_keys.push(aux);
            self.head.stake_amount.push(amount);
            self.head_total_stake += amount;
            self.bls_mapping.insert(new_key, pos);
            Ok(())
        }
    }

    fn deregister(&mut self, existing_key: &Self::Key) -> Result<(), StakeTableError> {
        match self.bls_mapping.get(existing_key) {
            Some(pos) => {
                self.head_total_stake -= self.head.stake_amount[*pos];
                self.head.stake_amount[*pos] = U256::ZERO;
                Ok(())
            },
            None => Err(StakeTableError::KeyNotFound),
        }
    }

    fn commitment(&self, version: SnapshotVersion) -> Result<Self::Commitment, StakeTableError> {
        match version {
            // IMPORTANT: we don't support committing the head version b/c it's not finalized.
            SnapshotVersion::EpochStart => Ok(self.epoch_start_comm),
            SnapshotVersion::LastEpochStart => Ok(self.last_epoch_start_comm),
            _ => Err(StakeTableError::SnapshotUnsupported),
        }
    }

    fn total_stake(&self, version: SnapshotVersion) -> Result<Self::Amount, StakeTableError> {
        match version {
            SnapshotVersion::Head => Ok(self.head_total_stake),
            SnapshotVersion::EpochStart => Ok(self.epoch_start_total_stake),
            SnapshotVersion::LastEpochStart => Ok(self.last_epoch_start_total_stake),
            SnapshotVersion::BlockNum(_) => Err(StakeTableError::SnapshotUnsupported),
        }
    }

    fn len(&self, version: SnapshotVersion) -> Result<usize, StakeTableError> {
        Ok(self.version(&version)?.bls_keys.len())
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        self.bls_mapping.contains_key(key)
    }

    fn lookup(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<Self::Amount, StakeTableError> {
        let table = self.version(&version)?;
        let pos = self.lookup_pos(key)?;
        if pos >= table.bls_keys.len() {
            Err(StakeTableError::KeyNotFound)
        } else {
            Ok(table.stake_amount[pos])
        }
    }

    fn lookup_with_proof(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<(Self::Amount, Self::LookupProof), StakeTableError> {
        let amount = self.lookup(version, key)?;
        Ok((amount, ()))
    }

    fn lookup_with_aux_and_proof(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<(Self::Amount, Self::Aux, Self::LookupProof), StakeTableError> {
        let table = self.version(&version)?;
        let pos = self.lookup_pos(key)?;
        if pos >= table.bls_keys.len() {
            Err(StakeTableError::KeyNotFound)
        } else {
            Ok((table.stake_amount[pos], table.schnorr_keys[pos].clone(), ()))
        }
    }

    fn update(
        &mut self,
        key: &Self::Key,
        delta: Self::Amount,
        negative: bool,
    ) -> Result<Self::Amount, StakeTableError> {
        let pos = self.lookup_pos(key)?;
        if negative {
            if delta > self.head.stake_amount[pos] {
                return Err(StakeTableError::InsufficientFund);
            }
            self.head_total_stake -= delta;
            self.head.stake_amount[pos] -= delta;
        } else {
            self.head_total_stake += delta;
            self.head.stake_amount[pos] += delta;
        }
        Ok(self.head.stake_amount[pos])
    }

    fn sample(
        &self,
        rng: &mut (impl SeedableRng + CryptoRngCore),
    ) -> Option<(&Self::Key, &Self::Amount)> {
        let mut bytes = [0u8; 64];
        rng.fill_bytes(&mut bytes);
        let r = U512::from_be_slice(&bytes);
        let m = U512::from(self.last_epoch_start_total_stake);
        let mut pos: U256 = (r % m).to::<U256>();
        let idx = 0;
        while pos > self.last_epoch_start.stake_amount[idx] {
            pos -= self.last_epoch_start.stake_amount[idx];
        }
        Some((
            &self.last_epoch_start.bls_keys[idx],
            &self.last_epoch_start.stake_amount[idx],
        ))
    }

    fn try_iter(&self, version: SnapshotVersion) -> Result<Self::IntoIter, StakeTableError> {
        let table = self.version(&version)?;
        let owned = (0..table.bls_keys.len())
            .map(|i| {
                (
                    table.bls_keys[i].clone(),
                    table.stake_amount[i],
                    table.schnorr_keys[i].clone(),
                )
            })
            .collect::<Vec<_>>();
        Ok(owned.into_iter())
    }
}

impl<K1, K2, F> StakeTable<K1, K2, F>
where
    K1: Eq + Hash + Clone + ToFields<F>,
    K2: Eq + Hash + Clone + Default + ToFields<F>,
    F: RescueParameter,
{
    /// Initiating an empty stake table.
    /// # Panics
    /// If unable to evaluate a preimage
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let bls_comm_preimage = vec![F::default(); capacity * <K1 as ToFields<F>>::SIZE];
        let default_bls_comm =
            VariableLengthRescueCRHF::<F, 1>::evaluate(&bls_comm_preimage).unwrap()[0];
        let schnorr_comm_preimage = vec![F::default(); capacity * <K2 as ToFields<F>>::SIZE];
        let default_schnorr_comm =
            VariableLengthRescueCRHF::<F, 1>::evaluate(&schnorr_comm_preimage).unwrap()[0];
        let stake_comm_preimage = vec![F::default(); capacity];
        let default_stake_comm =
            VariableLengthRescueCRHF::<F, 1>::evaluate(&stake_comm_preimage).unwrap()[0];
        let default_comm = (default_bls_comm, default_schnorr_comm, default_stake_comm);
        Self {
            capacity,
            head: StakeTableSnapshot::default(),
            epoch_start: StakeTableSnapshot::default(),
            last_epoch_start: StakeTableSnapshot::default(),
            head_total_stake: U256::ZERO,
            epoch_start_total_stake: U256::ZERO,
            last_epoch_start_total_stake: U256::ZERO,
            bls_mapping: HashMap::new(),
            epoch_start_comm: default_comm,
            last_epoch_start_comm: default_comm,
        }
    }

    /// Update the stake table when the epoch number advances, should be manually called.
    pub fn advance(&mut self) {
        // Could we avoid this `clone()`?
        self.last_epoch_start = self.epoch_start.clone();
        self.last_epoch_start_total_stake = self.epoch_start_total_stake;
        self.last_epoch_start_comm = self.epoch_start_comm;
        self.epoch_start = self.head.clone();
        self.epoch_start_total_stake = self.head_total_stake;
        self.epoch_start_comm = self.compute_head_comm();
    }

    /// Set the stake withheld by `key` to be `value`.
    /// Return the previous stake if succeed.
    /// # Errors
    /// Errors if key is not in the stake table
    pub fn set_value(&mut self, key: &K1, value: U256) -> Result<U256, StakeTableError> {
        match self.bls_mapping.get(key) {
            Some(pos) => {
                let old_value = self.head.stake_amount[*pos];
                self.head.stake_amount[*pos] = value;
                self.head_total_stake -= old_value;
                self.head_total_stake += value;
                Ok(old_value)
            },
            None => Err(StakeTableError::KeyNotFound),
        }
    }

    /// Returns the stake table state used for voting
    pub fn voting_state(&self) -> Result<GenericStakeTableState<F>, StakeTableError> {
        let (bls_key_comm, schnorr_key_comm, amount_comm) =
            self.commitment(SnapshotVersion::LastEpochStart)?;
        let threshold = one_honest_threshold(self.total_stake(SnapshotVersion::LastEpochStart)?);
        Ok(GenericStakeTableState {
            bls_key_comm,
            schnorr_key_comm,
            amount_comm,
            threshold: u256_to_field(&threshold),
        })
    }

    /// Returns the stake table state used for voting in the next epoch
    pub fn next_voting_state(&self) -> Result<GenericStakeTableState<F>, StakeTableError> {
        let (bls_key_comm, schnorr_key_comm, amount_comm) =
            self.commitment(SnapshotVersion::EpochStart)?;
        let threshold = one_honest_threshold(self.total_stake(SnapshotVersion::EpochStart)?);
        Ok(GenericStakeTableState {
            bls_key_comm,
            schnorr_key_comm,
            amount_comm,
            threshold: u256_to_field(&threshold),
        })
    }

    /// Helper function to recompute the stake table commitment for head version
    /// Commitment of a stake table is a triple `(bls_keys_comm, schnorr_keys_comm, stake_amount_comm)`
    /// TODO(Chengyu): The BLS verification keys doesn't implement Default. Thus we directly pad with `F::default()`.
    fn compute_head_comm(&mut self) -> (F, F, F) {
        let padding_len = self.capacity - self.head.bls_keys.len();
        // Compute rescue hash for bls keys
        let mut bls_comm_preimage = self
            .head
            .bls_keys
            .iter()
            .flat_map(ToFields::to_fields)
            .collect::<Vec<_>>();
        bls_comm_preimage.resize(self.capacity * <K1 as ToFields<F>>::SIZE, F::default());
        let bls_comm = VariableLengthRescueCRHF::<F, 1>::evaluate(bls_comm_preimage).unwrap()[0];

        // Compute rescue hash for Schnorr keys
        let schnorr_comm_preimage = self
            .head
            .schnorr_keys
            .iter()
            .chain(ark_std::iter::repeat_n(&K2::default(), padding_len))
            .flat_map(ToFields::to_fields)
            .collect::<Vec<_>>();
        let schnorr_comm =
            VariableLengthRescueCRHF::<F, 1>::evaluate(schnorr_comm_preimage).unwrap()[0];

        // Compute rescue hash for stake amounts
        let mut stake_comm_preimage = self
            .head
            .stake_amount
            .iter()
            .map(|x| u256_to_field(x))
            .collect::<Vec<_>>();
        stake_comm_preimage.resize(self.capacity, F::default());
        let stake_comm =
            VariableLengthRescueCRHF::<F, 1>::evaluate(stake_comm_preimage).unwrap()[0];
        (bls_comm, schnorr_comm, stake_comm)
    }

    /// Return the index of a given key.
    /// Err if the key doesn't exists
    fn lookup_pos(&self, key: &K1) -> Result<usize, StakeTableError> {
        match self.bls_mapping.get(key) {
            Some(pos) => Ok(*pos),
            None => Err(StakeTableError::KeyNotFound),
        }
    }

    /// returns the snapshot version
    fn version(
        &self,
        version: &SnapshotVersion,
    ) -> Result<&StakeTableSnapshot<K1, K2>, StakeTableError> {
        match version {
            SnapshotVersion::Head => Ok(&self.head),
            SnapshotVersion::EpochStart => Ok(&self.epoch_start),
            SnapshotVersion::LastEpochStart => Ok(&self.last_epoch_start),
            SnapshotVersion::BlockNum(_) => Err(StakeTableError::SnapshotUnsupported),
        }
    }
}

impl<K1, K2, F> Default for StakeTable<K1, K2, F>
where
    K1: Eq + Hash + Clone + ToFields<F>,
    K2: Eq + Hash + Clone + Default + ToFields<F>,
    F: RescueParameter,
{
    fn default() -> Self {
        Self::new(STAKE_TABLE_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use ark_std::{rand::SeedableRng, vec::Vec};
    use hotshot_types::traits::stake_table::{SnapshotVersion, StakeTableError, StakeTableScheme};
    use jf_signature::{
        bls_over_bn254::BLSOverBN254CurveSignatureScheme, schnorr::SchnorrSignatureScheme,
        SignatureScheme,
    };

    use super::{
        config::{FieldType as F, QCVerKey, StateVerKey},
        StakeTable,
    };

    #[test]
    fn crypto_test_stake_table() -> Result<(), StakeTableError> {
        let mut st = StakeTable::<QCVerKey, StateVerKey, F>::default();
        let mut pseudo_rng = jf_utils::test_rng();
        let keys = (0..10)
            .map(|_| {
                (
                    BLSOverBN254CurveSignatureScheme::key_gen(&(), &mut pseudo_rng)
                        .unwrap()
                        .1,
                    SchnorrSignatureScheme::key_gen(&(), &mut pseudo_rng)
                        .unwrap()
                        .1,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(st.total_stake(SnapshotVersion::Head)?, U256::from(0));

        // Registering keys
        keys.iter()
            .take(4)
            .for_each(|key| st.register(key.0, U256::from(100), key.1.clone()).unwrap());
        assert_eq!(st.total_stake(SnapshotVersion::Head)?, U256::from(400));
        assert_eq!(st.total_stake(SnapshotVersion::EpochStart)?, U256::from(0));
        assert_eq!(
            st.total_stake(SnapshotVersion::LastEpochStart)?,
            U256::from(0)
        );
        // set to zero for further sampling test
        assert_eq!(
            st.set_value(&keys[1].0, U256::from(0)).unwrap(),
            U256::from(100)
        );
        st.advance();
        keys.iter()
            .skip(4)
            .take(3)
            .for_each(|key| st.register(key.0, U256::from(100), key.1.clone()).unwrap());
        assert_eq!(st.total_stake(SnapshotVersion::Head)?, U256::from(600));
        assert_eq!(
            st.total_stake(SnapshotVersion::EpochStart)?,
            U256::from(300)
        );
        assert_eq!(
            st.total_stake(SnapshotVersion::LastEpochStart)?,
            U256::from(0)
        );
        st.advance();
        keys.iter()
            .skip(7)
            .for_each(|key| st.register(key.0, U256::from(100), key.1.clone()).unwrap());
        assert_eq!(st.total_stake(SnapshotVersion::Head)?, U256::from(900));
        assert_eq!(
            st.total_stake(SnapshotVersion::EpochStart)?,
            U256::from(600)
        );
        assert_eq!(
            st.total_stake(SnapshotVersion::LastEpochStart)?,
            U256::from(300)
        );

        // No duplicate register
        assert!(st
            .register(keys[0].0, U256::from(100), keys[0].1.clone())
            .is_err());
        // The 9-th key is still in head stake table
        assert!(st.lookup(SnapshotVersion::EpochStart, &keys[9].0).is_err());
        assert!(st.lookup(SnapshotVersion::EpochStart, &keys[5].0).is_ok());
        // The 6-th key is still frozen
        assert!(st
            .lookup(SnapshotVersion::LastEpochStart, &keys[6].0)
            .is_err());
        assert!(st
            .lookup(SnapshotVersion::LastEpochStart, &keys[2].0)
            .is_ok());

        // Set value shall return the old value
        assert_eq!(
            st.set_value(&keys[0].0, U256::from(101)).unwrap(),
            U256::from(100)
        );
        assert_eq!(st.total_stake(SnapshotVersion::Head)?, U256::from(901));
        assert_eq!(
            st.total_stake(SnapshotVersion::EpochStart)?,
            U256::from(600)
        );

        // Update that results in a negative stake
        assert!(st.update(&keys[0].0, U256::from(1000), true).is_err());
        // Update should return the updated stake
        assert_eq!(
            st.update(&keys[0].0, U256::from(1), true).unwrap(),
            U256::from(100)
        );
        assert_eq!(
            st.update(&keys[0].0, U256::from(100), false).unwrap(),
            U256::from(200)
        );

        // Commitment test
        assert!(st.commitment(SnapshotVersion::Head).is_err());
        assert!(st.commitment(SnapshotVersion::EpochStart).is_ok());
        assert!(st.commitment(SnapshotVersion::LastEpochStart).is_ok());

        // Random test for sampling keys
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(41u64);
        for _ in 0..100 {
            let (_key, value) = st.sample(&mut rng).unwrap();
            // Sampled keys should have positive stake
            assert!(value > &U256::from(0));
        }

        // Test for try_iter
        for (i, (k1, _, k2)) in st.try_iter(SnapshotVersion::Head).unwrap().enumerate() {
            assert_eq!((k1, k2), keys[i]);
        }

        Ok(())
    }
}
