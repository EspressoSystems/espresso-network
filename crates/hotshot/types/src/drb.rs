// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::BTreeMap, sync::Arc, time::Instant};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use vbs::version::Version;
use versions::DRB_AND_HEADER_UPGRADE_VERSION;

use crate::{
    HotShotConfig,
    data::EpochNumber,
    traits::{
        node_implementation::NodeType,
        storage::{LoadDrbProgressFn, StoreDrbProgressFn},
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrbInput {
    /// The epoch we are calculating the result for
    pub epoch: u64,
    /// The iteration this seed is from. For fresh calculations, this should be `0`.
    pub iteration: u64,
    /// the value of the drb calculation at the current iteration
    pub value: [u8; 32],
    /// difficulty value for the DRB calculation
    pub difficulty_level: u64,
}

pub type DrbDifficultySelectorFn =
    Arc<dyn Fn(Version) -> BoxFuture<'static, u64> + Send + Sync + 'static>;

pub fn drb_difficulty_selector<TYPES: NodeType>(
    config: &HotShotConfig<TYPES>,
) -> DrbDifficultySelectorFn {
    let base_difficulty = config.drb_difficulty;
    let upgrade_difficulty = config.drb_upgrade_difficulty;
    Arc::new(move |version| {
        Box::pin(async move {
            if version >= DRB_AND_HEADER_UPGRADE_VERSION {
                upgrade_difficulty
            } else {
                base_difficulty
            }
        })
    })
}

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
pub const DIFFICULTY_LEVEL: u64 = 10;

/// Interval at which to store the results
pub const DRB_CHECKPOINT_INTERVAL: u64 = 1_000_000_000;

/// Hashes between cancellation checks. Bounds how long hashing continues after `cancel`
/// fires; independent of the `DRB_CHECKPOINT_INTERVAL` persistence cadence.
const DRB_CANCEL_BATCH: u64 = 1_000_000;

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

/// Hash `hash` repeatedly `count` times, checking `cancel` between batches.
///
/// Returns `None` if cancelled, `Some(hash)` otherwise.
fn hash_batches(mut hash: [u8; 32], count: u64, cancel: CancellationToken) -> Option<[u8; 32]> {
    let mut done = 0u64;
    while done < count {
        if cancel.is_cancelled() {
            return None;
        }
        let n = DRB_CANCEL_BATCH.min(count - done);
        for _ in 0..n {
            hash = Sha256::digest(hash).into();
        }
        done += n;
    }
    Some(hash)
}

/// Compute the DRB result for the leader rotation.
///
/// This is to be started two epochs in advance and spawned in a non-blocking thread.
/// Returns `None` if the computation was cancelled via `cancel`.
///
/// # Arguments
/// * `drb_seed_input` - Serialized QC signature.
/// * `cancel` - Token that stops the hash loop when fired.
#[must_use]
pub async fn compute_drb_result(
    drb_input: DrbInput,
    store_drb_progress: StoreDrbProgressFn,
    load_drb_progress: LoadDrbProgressFn,
    cancel: CancellationToken,
) -> Option<DrbResult> {
    info!(target: "announce::drb", ?drb_input, "beginning drb calculation");
    let mut drb_input = drb_input;

    if let Ok(loaded_drb_input) = load_drb_progress(drb_input.epoch).await {
        if loaded_drb_input.difficulty_level != drb_input.difficulty_level {
            error!(
                ?drb_input,
                ?loaded_drb_input,
                "we are calculating the drb result with input that has a different difficulty \
                 level for this epoch than a previously stored one => discarding the value from \
                 storage"
            );
        } else if loaded_drb_input.iteration >= drb_input.iteration {
            drb_input = loaded_drb_input;
        }
    }

    let mut hash: [u8; 32] = drb_input.value;
    let mut iteration = drb_input.iteration;
    let remaining_iterations = drb_input
        .difficulty_level
        .checked_sub(iteration)
        .unwrap_or_else(|| {
            panic!(
                "DRB difficulty level {} exceeds the iteration {} of the input we were given. \
                 This is a fatal error",
                drb_input.difficulty_level, iteration
            )
        });

    let final_checkpoint = remaining_iterations / DRB_CHECKPOINT_INTERVAL;

    let mut last_time = Instant::now();
    let mut last_iteration = iteration;

    // loop up to, but not including, the `final_checkpoint`
    for _ in 0..final_checkpoint {
        let c = cancel.clone();
        hash = tokio::task::spawn_blocking(move || hash_batches(hash, DRB_CHECKPOINT_INTERVAL, c))
            .await
            .expect("completes")?;

        iteration += DRB_CHECKPOINT_INTERVAL;

        let updated_drb_input = DrbInput {
            epoch: drb_input.epoch,
            iteration,
            value: hash,
            difficulty_level: drb_input.difficulty_level,
        };

        let elapsed_time = last_time.elapsed().as_millis();

        let store_drb_progress = store_drb_progress.clone();
        tokio::spawn(async move {
            info!(
                target: "announce::drb",
                ?updated_drb_input,
                %last_iteration,
                %elapsed_time,
                "storing partial drb progress"
            );
            if let Err(err) = store_drb_progress(updated_drb_input).await {
                warn!(%err, "failed to store drb progress during calculation");
            }
        });

        last_time = Instant::now();
        last_iteration = iteration;
    }

    let final_checkpoint_iteration = iteration;
    // Holds by construction: `iteration` only ever reaches multiples of the checkpoint
    // interval not exceeding `difficulty_level`.
    let remainder = drb_input
        .difficulty_level
        .checked_sub(final_checkpoint_iteration)
        .expect("sufficient iterations");

    // perform the remaining iterations
    let c = cancel.clone();
    let drb_result = tokio::task::spawn_blocking(move || hash_batches(hash, remainder, c))
        .await
        .expect("DRB spawn_blocking panicked")?;

    let final_drb_input = DrbInput {
        epoch: drb_input.epoch,
        iteration: drb_input.difficulty_level,
        value: drb_result,
        difficulty_level: drb_input.difficulty_level,
    };

    info!(target: "announce::drb", ?final_drb_input, "completed drb calculation");

    let store_drb_progress = store_drb_progress.clone();
    tokio::spawn(async move {
        if let Err(err) = store_drb_progress(final_drb_input).await {
            warn!(%err, "failed to store drb progress during calculation");
        }
    });

    Some(drb_result)
}

/// Seeds for DRB computation and computed results.
#[derive(Clone, Debug)]
pub struct DrbResults {
    /// Stored results from computations
    pub results: BTreeMap<EpochNumber, DrbResult>,
}

impl DrbResults {
    #[must_use]
    /// Constructor with initial values for epochs 1 and 2.
    pub fn new() -> Self {
        Self {
            results: BTreeMap::from([
                (EpochNumber::new(1), INITIAL_DRB_RESULT),
                (EpochNumber::new(2), INITIAL_DRB_RESULT),
            ]),
        }
    }

    pub fn store_result(&mut self, epoch: EpochNumber, result: DrbResult) {
        self.results.insert(epoch, result);
    }

    /// Garbage collects internal data structures
    pub fn garbage_collect(&mut self, epoch: EpochNumber) {
        if epoch.u64() < KEEP_PREVIOUS_RESULT_COUNT {
            return;
        }

        let retain_epoch = epoch - KEEP_PREVIOUS_RESULT_COUNT;
        // N.B. x.split_off(y) returns the part of the map where key >= y

        // Remove result entries older than EPOCH
        self.results = self.results.split_off(&retain_epoch);
    }
}

impl Default for DrbResults {
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
        let breakpoint: U256 = U256::from_le_slice(&remainder.to_le_bytes_vec()[0..32]);

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

    impl<Entry> RandomizedCommittee<Entry> {
        pub fn drb_result(&self) -> [u8; 32] {
            self.drb
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use alloy::primitives::U256;
    use rand::RngCore;
    use sha2::{Digest, Sha256};
    use tokio_util::sync::CancellationToken;

    use super::{
        DRB_CANCEL_BATCH, DRB_CHECKPOINT_INTERVAL, DrbInput, compute_drb_result,
        election::{generate_stake_cdf, select_randomized_leader},
        hash_batches,
    };
    use crate::{
        signature_key::BLSPubKey,
        stake_table::StakeTableEntry,
        traits::{
            signature_key::{BuilderSignatureKey, StakeTableEntryType},
            storage::{null_load_drb_progress_fn, null_store_drb_progress_fn},
        },
    };

    #[test]
    fn test_hash_batches_pre_cancelled() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(hash_batches([0u8; 32], 10, cancel), None);
    }

    #[test]
    fn test_hash_batches_count_zero() {
        let input = [42u8; 32];
        let result = hash_batches(input, 0, CancellationToken::new());
        assert_eq!(result, Some(input));
    }

    #[test]
    fn test_hash_batches_small_count() {
        let input = [1u8; 32];
        let count = 5u64;
        // count < DRB_CANCEL_BATCH, so a single batch completes
        let mut expected = input;
        for _ in 0..count {
            expected = Sha256::digest(expected).into();
        }
        let result = hash_batches(input, count, CancellationToken::new());
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_hash_batches_cancel_after_first_batch() {
        // count spans 2 batches; cancel after first batch boundary
        let count = DRB_CANCEL_BATCH + 1;
        let cancel = CancellationToken::new();

        // Compute one full batch, then cancel, then try to hash the remainder
        let intermediate = hash_batches([0u8; 32], DRB_CANCEL_BATCH, CancellationToken::new())
            .expect("first batch must complete");
        cancel.cancel();
        // The remaining 1 iteration sees cancelled token on first check
        let result = hash_batches(intermediate, count - DRB_CANCEL_BATCH, cancel);
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_compute_drb_result_cancelled() {
        // A pre-cancelled token must abandon a multi-checkpoint computation
        // immediately rather than running it to completion.
        let cancel = CancellationToken::new();
        cancel.cancel();
        let drb_input = DrbInput {
            epoch: 1,
            iteration: 0,
            value: [0u8; 32],
            difficulty_level: DRB_CHECKPOINT_INTERVAL * 5,
        };
        let result = compute_drb_result(
            drb_input,
            null_store_drb_progress_fn(),
            null_load_drb_progress_fn(),
            cancel,
        )
        .await;
        assert_eq!(result, None);
    }

    #[test]
    fn test_randomized_leader() {
        let mut rng = rand::thread_rng();
        // use an arbitrary Sha256 output.
        let drb: [u8; 32] = Sha256::digest(b"drb").into();
        // a stake table with 10 nodes, each with a stake of 1-100
        let stake_table_entries: Vec<_> = (0..10)
            .map(|i| StakeTableEntry {
                stake_key: BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0,
                stake_amount: U256::from(rng.next_u64() % 100 + 1),
            })
            .collect();
        let randomized_committee = generate_stake_cdf(stake_table_entries.clone(), drb);

        // Number of views to test
        let num_views = 100000;
        let mut selected = HashMap::<_, u64>::new();
        // Test the leader election for 100000 views.
        for i in 0..num_views {
            let leader = select_randomized_leader(&randomized_committee, i);
            *selected.entry(leader).or_insert(0) += 1;
        }

        // Total variation distance
        let mut tvd = 0.;
        let total_stakes = stake_table_entries
            .iter()
            .map(|e| e.stake())
            .sum::<U256>()
            .to::<u64>() as f64;
        for entry in stake_table_entries {
            let expected = entry.stake().to::<u64>() as f64 / total_stakes;
            let actual = *selected.get(&entry).unwrap_or(&0) as f64 / num_views as f64;
            tvd += (expected - actual).abs();
        }

        // sanity check
        assert!(tvd >= 0.0);
        // Allow a small margin of error
        assert!(tvd < 0.03);
    }
}
