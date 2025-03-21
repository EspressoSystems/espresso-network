// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Trait for stake table data structures

use ark_std::{rand::SeedableRng, vec::Vec};
use digest::crypto_common::rand_core::CryptoRngCore;
use displaydoc::Display;

/// Snapshots of the stake table
pub enum SnapshotVersion {
    /// the latest "Head" where all new changes are applied to
    Head,
    /// marks the snapshot at the beginning of the current epoch
    EpochStart,
    /// marks the beginning of the last epoch
    LastEpochStart,
    /// at arbitrary block height
    BlockNum(u64),
}

/// Common interfaces required for a stake table used in `HotShot` System.
/// APIs that doesn't take `version: SnapshotVersion` as an input by default works on the head/latest version.
pub trait StakeTableScheme {
    /// type for stake key
    type Key: Clone;
    /// type for the staked amount
    type Amount: Clone + Copy;
    /// type for the commitment to the current stake table
    type Commitment;
    /// type for the proof associated with the lookup result (if any)
    type LookupProof;
    /// type for the iterator over (key, value) entries
    type IntoIter: Iterator<Item = (Self::Key, Self::Amount, Self::Aux)>;
    /// Auxiliary information associated with the key
    type Aux: Clone;

    /// Register a new key into the stake table.
    ///
    /// # Errors
    ///
    /// Return err if key is already registered.
    fn register(
        &mut self,
        new_key: Self::Key,
        amount: Self::Amount,
        aux: Self::Aux,
    ) -> Result<(), StakeTableError>;

    /// Batch register a list of new keys. A default implementation is provided
    /// w/o batch optimization.
    ///
    /// # Errors
    ///
    /// Return err if any of `new_keys` fails to register.
    fn batch_register<I, J, K>(
        &mut self,
        new_keys: I,
        amounts: J,
        auxs: K,
    ) -> Result<(), StakeTableError>
    where
        I: IntoIterator<Item = Self::Key>,
        J: IntoIterator<Item = Self::Amount>,
        K: IntoIterator<Item = Self::Aux>,
    {
        let _ = new_keys
            .into_iter()
            .zip(amounts)
            .zip(auxs)
            .try_for_each(|((key, amount), aux)| Self::register(self, key, amount, aux));
        Ok(())
    }

    /// Deregister an existing key from the stake table.
    /// Returns error if some keys are not found.
    ///
    /// # Errors
    /// Return err if `existing_key` wasn't registered.
    fn deregister(&mut self, existing_key: &Self::Key) -> Result<(), StakeTableError>;

    /// Batch deregister a list of keys. A default implementation is provided
    /// w/o batch optimization.
    ///
    /// # Errors
    /// Return err if any of `existing_keys` fail to deregister.
    fn batch_deregister<'a, I>(&mut self, existing_keys: I) -> Result<(), StakeTableError>
    where
        I: IntoIterator<Item = &'a <Self as StakeTableScheme>::Key>,
        <Self as StakeTableScheme>::Key: 'a,
    {
        let _ = existing_keys
            .into_iter()
            .try_for_each(|key| Self::deregister(self, key));
        Ok(())
    }

    /// Returns the commitment to the `version` of stake table.
    ///
    /// # Errors
    /// Return err if the `version` is not supported.
    fn commitment(&self, version: SnapshotVersion) -> Result<Self::Commitment, StakeTableError>;

    /// Returns the accumulated stakes of all registered keys of the `version`
    /// of stake table.
    ///
    /// # Errors
    /// Return err if the `version` is not supported.
    fn total_stake(&self, version: SnapshotVersion) -> Result<Self::Amount, StakeTableError>;

    /// Returns the number of keys in the `version` of the table.
    ///
    /// # Errors
    /// Return err if the `version` is not supported.
    fn len(&self, version: SnapshotVersion) -> Result<usize, StakeTableError>;

    /// Returns true if `key` is currently registered, else returns false.
    fn contains_key(&self, key: &Self::Key) -> bool;

    /// Returns the stakes withhelded by a public key.
    ///
    /// # Errors
    /// Return err if the `version` is not supported or `key` doesn't exist.
    fn lookup(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<Self::Amount, StakeTableError>;

    /// Returns the stakes withhelded by a public key along with a membership proof.
    ///
    /// # Errors
    /// Return err if the `version` is not supported or `key` doesn't exist.
    fn lookup_with_proof(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<(Self::Amount, Self::LookupProof), StakeTableError>;

    /// Return the associated stake amount and auxiliary information of a public key,
    /// along with a membership proof.
    ///
    /// # Errors
    /// Return err if the `version` is not supported or `key` doesn't exist.
    #[allow(clippy::type_complexity)]
    fn lookup_with_aux_and_proof(
        &self,
        version: SnapshotVersion,
        key: &Self::Key,
    ) -> Result<(Self::Amount, Self::Aux, Self::LookupProof), StakeTableError>;

    /// Update the stake of the `key` with `(negative ? -1 : 1) * delta`.
    /// Return the updated stake or error.
    ///
    /// # Errors
    /// Return err if the `key` doesn't exist of if the update overflow/underflow.
    fn update(
        &mut self,
        key: &Self::Key,
        delta: Self::Amount,
        negative: bool,
    ) -> Result<Self::Amount, StakeTableError>;

    /// Batch update the stake balance of `keys`. Read documentation about
    /// [`Self::update()`]. By default, we call `Self::update()` on each
    /// (key, amount, negative) tuple.
    ///
    /// # Errors
    /// Return err if any one of the `update` failed.
    fn batch_update(
        &mut self,
        keys: &[Self::Key],
        amounts: &[Self::Amount],
        negative_flags: Vec<bool>,
    ) -> Result<Vec<Self::Amount>, StakeTableError> {
        let updated_amounts = keys
            .iter()
            .zip(amounts.iter())
            .zip(negative_flags.iter())
            .map(|((key, &amount), negative)| Self::update(self, key, amount, *negative))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(updated_amounts)
    }

    /// Randomly sample a (key, stake amount) pair proportional to the stake distributions,
    /// given a fixed seed for `rng`, this sampling should be deterministic.
    fn sample(
        &self,
        rng: &mut (impl SeedableRng + CryptoRngCore),
    ) -> Option<(&Self::Key, &Self::Amount)>;

    /// Returns an iterator over all (key, value) entries of the `version` of the table
    ///
    /// # Errors
    /// Return err if the `version` is not supported.
    fn try_iter(&self, version: SnapshotVersion) -> Result<Self::IntoIter, StakeTableError>;
}

/// Error type for [`StakeTableScheme`]
#[derive(Debug, Display)]
pub enum StakeTableError {
    /// Internal error caused by Rescue
    RescueError,
    /// Key mismatched
    MismatchedKey,
    /// Key not found
    KeyNotFound,
    /// Key already exists
    ExistingKey,
    /// Malformed Merkle proof
    MalformedProof,
    /// Verification Error
    VerificationError,
    /// Insufficient fund: the number of stake cannot be negative
    InsufficientFund,
    /// The number of stake exceed U256
    StakeOverflow,
    /// The historical snapshot requested is not supported.
    SnapshotUnsupported,
}

impl ark_std::error::Error for StakeTableError {}
