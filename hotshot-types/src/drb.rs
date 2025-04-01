// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::traits::node_implementation::{ConsensusTime, NodeType};

// TODO: Add the following consts once we bench the hash time.
// <https://github.com/EspressoSystems/HotShot/issues/3880>
// /// Highest number of hashes that a hardware can complete in a second.
// const `HASHES_PER_SECOND`
// /// Time a DRB calculation will take, in terms of number of views.
// const `DRB_CALCULATION_NUM_VIEW`: u64 = 300;

// TODO: Replace this with an accurate number calculated by `fn difficulty_level()` once we bench
// the hash time.
// <https://github.com/EspressoSystems/HotShot/issues/3880>
/// Arbitrary number of times the hash function will be repeatedly called.
const DIFFICULTY_LEVEL: u64 = 10;

/// DRB seed input for epoch 1 and 2.
pub const INITIAL_DRB_SEED_INPUT: [u8; 32] = [0; 32];
/// DRB result for epoch 1 and 2.
pub const INITIAL_DRB_RESULT: [u8; 32] = [0; 32];

/// Alias for DRB seed input for `compute_drb_result`, serialized from the QC signature.
pub type DrbSeedInput = [u8; 32];

/// Alias for DRB result from `compute_drb_result`.
pub type DrbResult = [u8; 32];

/// Number of previous results and seeds to keep
pub const KEEP_PREVIOUS_RESULT_COUNT: u64 = 8;

// TODO: Use `HASHES_PER_SECOND` * `VIEW_TIMEOUT` * `DRB_CALCULATION_NUM_VIEW` to calculate this
// once we bench the hash time.
// <https://github.com/EspressoSystems/HotShot/issues/3880>
/// Difficulty level of the DRB calculation.
///
/// Represents the number of times the hash function will be repeatedly called.
#[must_use]
pub fn difficulty_level() -> u64 {
    unimplemented!("Use an arbitrary `DIFFICULTY_LEVEL` for now before we bench the hash time.");
}

/// Compute the DRB result for the leader rotation.
///
/// This is to be started two epochs in advance and spawned in a non-blocking thread.
///
/// # Arguments
/// * `drb_seed_input` - Serialized QC signature.
#[must_use]
pub fn compute_drb_result<TYPES: NodeType>(drb_seed_input: DrbSeedInput) -> DrbResult {
    let mut hash = drb_seed_input.to_vec();
    for _iter in 0..DIFFICULTY_LEVEL {
        // TODO: This may be optimized to avoid memcopies after we bench the hash time.
        // <https://github.com/EspressoSystems/HotShot/issues/3880>
        hash = Sha256::digest(hash).to_vec();
    }

    // Convert the hash to the DRB result.
    let mut drb_result = [0u8; 32];
    drb_result.copy_from_slice(&hash);
    drb_result
}

/// Seeds for DRB computation and computed results.
#[derive(Clone, Debug)]
pub struct DrbResults<TYPES: NodeType> {
    /// Stored results from computations
    pub results: BTreeMap<TYPES::Epoch, DrbResult>,
}

impl<TYPES: NodeType> DrbResults<TYPES> {
    #[must_use]
    /// Constructor with initial values for epochs 1 and 2.
    pub fn new() -> Self {
        Self {
            results: BTreeMap::from([
                (TYPES::Epoch::new(1), INITIAL_DRB_RESULT),
                (TYPES::Epoch::new(2), INITIAL_DRB_RESULT),
            ]),
        }
    }

    pub fn store_result(&mut self, epoch: TYPES::Epoch, result: DrbResult) {
        self.results.insert(epoch, result);
    }

    /// Garbage collects internal data structures
    pub fn garbage_collect(&mut self, epoch: TYPES::Epoch) {
        if epoch.u64() < KEEP_PREVIOUS_RESULT_COUNT {
            return;
        }

        let retain_epoch = epoch - KEEP_PREVIOUS_RESULT_COUNT;
        // N.B. x.split_off(y) returns the part of the map where key >= y

        // Remove result entries older than EPOCH
        self.results = self.results.split_off(&retain_epoch);
    }
}

impl<TYPES: NodeType> Default for DrbResults<TYPES> {
    fn default() -> Self {
        Self::new()
    }
}

/// Functions for leader selection based on the DRB.
///
/// The algorithm we use is:
///
/// Initialization:
/// - obtain `drb: [u8; 32]` from the DRB calculation
/// - sort the stake table for a given epoch by `xor(drb, public_key)`
/// - generate a cdf of the cumulative stake using this newly-sorted table,
///   along with a hash of the stake table entries
///
/// Selecting a leader:
/// - calculate the SHA512 hash of the `drb_result`, `view_number` and `stake_table_hash`
/// - find the first index in the cdf for which the remainder of this hash modulo the `total_stake`
///   is strictly smaller than the cdf entry
/// - return the corresponding node as the leader for that view
pub mod election {
    use alloy::primitives::{U256, U512};
    use sha2::{Digest, Sha256, Sha512};

    use crate::traits::signature_key::{SignatureKey, StakeTableEntryType};

    /// Calculate `xor(drb.cycle(), public_key)`, returning the result as a vector of bytes
    fn cyclic_xor(drb: [u8; 32], public_key: Vec<u8>) -> Vec<u8> {
        let drb: Vec<u8> = drb.to_vec();

        let mut result: Vec<u8> = vec![];

        for (drb_byte, public_key_byte) in public_key.iter().zip(drb.iter().cycle()) {
            result.push(drb_byte ^ public_key_byte);
        }

        result
    }

    /// Generate the stake table CDF, as well as a hash of the resulting stake table
    pub fn generate_stake_cdf<Key: SignatureKey, Entry: StakeTableEntryType<Key>>(
        mut stake_table: Vec<Entry>,
        drb: [u8; 32],
    ) -> RandomizedCommittee<Entry> {
        // sort by xor(public_key, drb_result)
        stake_table.sort_by(|a, b| {
            cyclic_xor(drb, a.public_key().to_bytes())
                .cmp(&cyclic_xor(drb, b.public_key().to_bytes()))
        });

        let mut hasher = Sha256::new();

        let mut cumulative_stake = U256::from(0);
        let mut cdf = vec![];

        for entry in stake_table {
            cumulative_stake += entry.stake();
            hasher.update(entry.public_key().to_bytes());

            cdf.push((entry, cumulative_stake));
        }

        RandomizedCommittee {
            cdf,
            stake_table_hash: hasher.finalize().into(),
            drb,
        }
    }

    /// select the leader for a view
    ///
    /// # Panics
    /// Panics if `cdf` is empty. Results in undefined behaviour if `cdf` is not ordered.
    ///
    /// Note that we try to downcast a U512 to a U256,
    /// but this should never panic because the U512 should be strictly smaller than U256::MAX by construction.
    pub fn select_randomized_leader<
        SignatureKey,
        Entry: StakeTableEntryType<SignatureKey> + Clone,
    >(
        randomized_committee: &RandomizedCommittee<Entry>,
        view: u64,
    ) -> Entry {
        let RandomizedCommittee {
            cdf,
            stake_table_hash,
            drb,
        } = randomized_committee;
        // We hash the concatenated drb, view and stake table hash.
        let mut hasher = Sha512::new();
        hasher.update(drb);
        hasher.update(view.to_le_bytes());
        hasher.update(stake_table_hash);
        let raw_breakpoint: [u8; 64] = hasher.finalize().into();

        // then calculate the remainder modulo the total stake as a U512
        let remainder: U512 =
            U512::from_le_bytes(raw_breakpoint) % U512::from(cdf.last().unwrap().1);

        // and drop the top 32 bytes, downcasting to a U256
        let breakpoint: U256 = U256::from_le_slice(&remainder.to_le_bytes_vec()[32..]);

        // now find the first index where the breakpoint is strictly smaller than the cdf
        //
        // in principle, this may result in an index larger than `cdf.len()`.
        // however, we have ensured by construction that `breakpoint < total_stake`
        // and so the largest index we can actually return is `cdf.len() - 1`
        let index = cdf.partition_point(|(_, cumulative_stake)| breakpoint >= *cumulative_stake);

        // and return the corresponding entry
        cdf[index].0.clone()
    }

    #[derive(Clone, Debug)]
    pub struct RandomizedCommittee<Entry> {
        /// cdf of nodes by cumulative stake
        cdf: Vec<(Entry, U256)>,
        /// Hash of the stake table
        stake_table_hash: [u8; 32],
        /// DRB result
        drb: [u8; 32],
    }
}
