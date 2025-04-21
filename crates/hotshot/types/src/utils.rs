// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Utility functions, type aliases, helper structs and enum definitions.

use std::{
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

use alloy::primitives::U256;
use anyhow::{anyhow, ensure};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bincode::{
    config::{
        FixintEncoding, LittleEndian, RejectTrailing, WithOtherEndian, WithOtherIntEncoding,
        WithOtherLimit, WithOtherTrailing,
    },
    DefaultOptions, Options,
};
use committable::{Commitment, Committable};
use digest::OutputSizeUser;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tagged_base64::tagged;
use typenum::Unsigned;
use vbs::version::StaticVersionType;

use crate::{
    data::{Leaf2, VidCommitment},
    traits::{
        node_implementation::{ConsensusTime, NodeType, Versions},
        ValidatedState,
    },
    vote::{Certificate, HasViewNumber},
    PeerConfig, StakeTableEntries,
};

/// A view's state
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(bound = "")]
pub enum ViewInner<TYPES: NodeType> {
    /// A pending view with an available block but not leaf proposal yet.
    ///
    /// Storing this state allows us to garbage collect blocks for views where a proposal is never
    /// made. This saves memory when a leader fails and subverts a DoS attack where malicious
    /// leaders repeatedly request availability for blocks that they never propose.
    Da {
        /// Payload commitment to the available block.
        payload_commitment: VidCommitment,
        /// An epoch to which the data belongs to. Relevant for validating against the correct stake table
        epoch: Option<TYPES::Epoch>,
    },
    /// Undecided view
    Leaf {
        /// Proposed leaf
        leaf: LeafCommitment<TYPES>,
        /// Validated state.
        state: Arc<TYPES::ValidatedState>,
        /// Optional state delta.
        delta: Option<Arc<<TYPES::ValidatedState as ValidatedState<TYPES>>::Delta>>,
        /// An epoch to which the data belongs to. Relevant for validating against the correct stake table
        epoch: Option<TYPES::Epoch>,
    },
    /// Leaf has failed
    Failed,
}
impl<TYPES: NodeType> Clone for ViewInner<TYPES> {
    fn clone(&self) -> Self {
        match self {
            Self::Da {
                payload_commitment,
                epoch,
            } => Self::Da {
                payload_commitment: *payload_commitment,
                epoch: *epoch,
            },
            Self::Leaf {
                leaf,
                state,
                delta,
                epoch,
            } => Self::Leaf {
                leaf: *leaf,
                state: Arc::clone(state),
                delta: delta.clone(),
                epoch: *epoch,
            },
            Self::Failed => Self::Failed,
        }
    }
}
/// The hash of a leaf.
pub type LeafCommitment<TYPES> = Commitment<Leaf2<TYPES>>;

/// Optional validated state and state delta.
pub type StateAndDelta<TYPES> = (
    Option<Arc<<TYPES as NodeType>::ValidatedState>>,
    Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
);

pub async fn verify_leaf_chain<T: NodeType, V: Versions>(
    leaf_chain: Vec<Leaf2<T>>,
    stake_table: Vec<PeerConfig<T>>,
    success_threshold: U256,
    expected_height: u64,
    upgrade_lock: &crate::message::UpgradeLock<T, V>,
) -> anyhow::Result<Leaf2<T>> {
    // Check we actually have a chain long enough for deciding
    if leaf_chain.len() < 3 {
        return Err(anyhow!("Leaf chain is not long enough for a decide"));
    }

    let newest_leaf = leaf_chain.first().unwrap();
    let parent = &leaf_chain[1];
    let grand_parent = &leaf_chain[2];

    // Check if the leaves form a decide
    if newest_leaf.justify_qc().view_number() != parent.view_number()
        || parent.justify_qc().view_number() != grand_parent.view_number()
    {
        return Err(anyhow!("Leaf views do not chain"));
    }
    if newest_leaf.justify_qc().data.leaf_commit != parent.commit()
        || parent.justify_qc().data().leaf_commit != grand_parent.commit()
    {
        return Err(anyhow!("Leaf commits do not chain"));
    }
    if parent.view_number() != grand_parent.view_number() + 1 {
        return Err(anyhow::anyhow!(
            "Decide rule failed, parent does not directly extend grandparent"
        ));
    }

    // verify all QCs are valid
    newest_leaf
        .justify_qc()
        .is_valid_cert(
            StakeTableEntries::<T>::from(stake_table.clone()).0,
            success_threshold,
            upgrade_lock,
        )
        .await?;
    parent
        .justify_qc()
        .is_valid_cert(
            StakeTableEntries::<T>::from(stake_table.clone()).0,
            success_threshold,
            upgrade_lock,
        )
        .await?;
    grand_parent
        .justify_qc()
        .is_valid_cert(
            StakeTableEntries::<T>::from(stake_table.clone()).0,
            success_threshold,
            upgrade_lock,
        )
        .await?;

    // Verify the root is in the chain of decided leaves
    let mut last_leaf = parent;
    for leaf in leaf_chain.iter().skip(2) {
        ensure!(last_leaf.justify_qc().view_number() == leaf.view_number());
        ensure!(last_leaf.justify_qc().data().leaf_commit == leaf.commit());
        leaf.justify_qc()
            .is_valid_cert(
                StakeTableEntries::<T>::from(stake_table.clone()).0,
                success_threshold,
                upgrade_lock,
            )
            .await?;
        if leaf.height() == expected_height {
            return Ok(leaf.clone());
        }
        last_leaf = leaf;
    }
    Err(anyhow!("Epoch Root was not found in the decided chain"))
}

impl<TYPES: NodeType> ViewInner<TYPES> {
    /// Return the underlying undecide leaf commitment and validated state if they exist.
    #[must_use]
    pub fn leaf_and_state(&self) -> Option<(LeafCommitment<TYPES>, &Arc<TYPES::ValidatedState>)> {
        if let Self::Leaf { leaf, state, .. } = self {
            Some((*leaf, state))
        } else {
            None
        }
    }

    /// return the underlying leaf hash if it exists
    #[must_use]
    pub fn leaf_commitment(&self) -> Option<LeafCommitment<TYPES>> {
        if let Self::Leaf { leaf, .. } = self {
            Some(*leaf)
        } else {
            None
        }
    }

    /// return the underlying validated state if it exists
    #[must_use]
    pub fn state(&self) -> Option<&Arc<TYPES::ValidatedState>> {
        if let Self::Leaf { state, .. } = self {
            Some(state)
        } else {
            None
        }
    }

    /// Return the underlying validated state and state delta if they exist.
    #[must_use]
    pub fn state_and_delta(&self) -> StateAndDelta<TYPES> {
        if let Self::Leaf { state, delta, .. } = self {
            (Some(Arc::clone(state)), delta.clone())
        } else {
            (None, None)
        }
    }

    /// return the underlying block payload commitment if it exists
    #[must_use]
    pub fn payload_commitment(&self) -> Option<VidCommitment> {
        if let Self::Da {
            payload_commitment, ..
        } = self
        {
            Some(*payload_commitment)
        } else {
            None
        }
    }

    /// Returns `Epoch` if possible
    // #3967 REVIEW NOTE: This type is kinda ugly, should we Result<Option<Epoch>> instead?
    pub fn epoch(&self) -> Option<Option<TYPES::Epoch>> {
        match self {
            Self::Da { epoch, .. } | Self::Leaf { epoch, .. } => Some(*epoch),
            Self::Failed => None,
        }
    }
}

impl<TYPES: NodeType> Deref for View<TYPES> {
    type Target = ViewInner<TYPES>;

    fn deref(&self) -> &Self::Target {
        &self.view_inner
    }
}

/// This exists so we can perform state transitions mutably
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct View<TYPES: NodeType> {
    /// The view data. Wrapped in a struct so we can mutate
    pub view_inner: ViewInner<TYPES>,
}

/// A struct containing information about a finished round.
#[derive(Debug, Clone)]
pub struct RoundFinishedEvent<TYPES: NodeType> {
    /// The round that finished
    pub view_number: TYPES::View,
}

/// Whether or not to stop inclusively or exclusively when walking
#[derive(Copy, Clone, Debug)]
pub enum Terminator<T> {
    /// Stop right before this view number
    Exclusive(T),
    /// Stop including this view number
    Inclusive(T),
}

/// Type alias for byte array of SHA256 digest length
type Sha256Digest = [u8; <sha2::Sha256 as OutputSizeUser>::OutputSize::USIZE];

#[tagged("BUILDER_COMMITMENT")]
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
/// Commitment that builders use to sign block options.
/// A thin wrapper around a Sha256 digest.
pub struct BuilderCommitment(Sha256Digest);

impl BuilderCommitment {
    /// Create new commitment for `data`
    pub fn from_bytes(data: impl AsRef<[u8]>) -> Self {
        Self(sha2::Sha256::digest(data.as_ref()).into())
    }

    /// Create a new commitment from a raw Sha256 digest
    pub fn from_raw_digest(digest: impl Into<Sha256Digest>) -> Self {
        Self(digest.into())
    }
}

impl AsRef<Sha256Digest> for BuilderCommitment {
    fn as_ref(&self) -> &Sha256Digest {
        &self.0
    }
}

/// For the wire format, we use bincode with the following options:
///   - No upper size limit
///   - Little endian encoding
///   - Varint encoding
///   - Reject trailing bytes
#[allow(clippy::type_complexity)]
#[must_use]
#[allow(clippy::type_complexity)]
pub fn bincode_opts() -> WithOtherTrailing<
    WithOtherIntEncoding<
        WithOtherEndian<WithOtherLimit<DefaultOptions, bincode::config::Infinite>, LittleEndian>,
        FixintEncoding,
    >,
    RejectTrailing,
> {
    bincode::DefaultOptions::new()
        .with_no_limit()
        .with_little_endian()
        .with_fixint_encoding()
        .reject_trailing_bytes()
}

/// Returns an epoch number given a block number and an epoch height
#[must_use]
pub fn epoch_from_block_number(block_number: u64, epoch_height: u64) -> u64 {
    if epoch_height == 0 {
        0
    } else if block_number == 0 {
        1
    } else if block_number % epoch_height == 0 {
        block_number / epoch_height
    } else {
        block_number / epoch_height + 1
    }
}

/// Returns the block number of the epoch root in the given epoch
///
/// WARNING: This is NOT the root block for the given epoch.
/// To find that root block number for epoch e, call `root_block_in_epoch(e-2,_)`.
#[must_use]
pub fn root_block_in_epoch(epoch: u64, epoch_height: u64) -> u64 {
    if epoch_height == 0 || epoch < 1 {
        0
    } else {
        epoch_height * epoch - 5
    }
}

/// Get the block height of the transition block for the given epoch
#[must_use]
pub fn transition_block_for_epoch(epoch: u64, epoch_height: u64) -> u64 {
    if epoch_height == 0 || epoch < 1 {
        0
    } else {
        epoch_height * epoch - 3
    }
}

/// Returns an `Option<Epoch>` based on a boolean condition of whether or not epochs are enabled, a block number,
/// and the epoch height. If epochs are disabled or the epoch height is zero, returns None.
#[must_use]
pub fn option_epoch_from_block_number<TYPES: NodeType>(
    with_epoch: bool,
    block_number: u64,
    epoch_height: u64,
) -> Option<TYPES::Epoch> {
    if with_epoch {
        if epoch_height == 0 {
            None
        } else if block_number == 0 {
            Some(1u64)
        } else if block_number % epoch_height == 0 {
            Some(block_number / epoch_height)
        } else {
            Some(block_number / epoch_height + 1)
        }
        .map(TYPES::Epoch::new)
    } else {
        None
    }
}

/// Returns Some(1) if epochs are enabled by V::Base, otherwise returns None
#[must_use]
pub fn genesis_epoch_from_version<V: Versions, TYPES: NodeType>() -> Option<TYPES::Epoch> {
    (V::Base::VERSION >= V::Epochs::VERSION).then(|| TYPES::Epoch::new(1))
}

/// A function for generating a cute little user mnemonic from a hash
#[must_use]
pub fn mnemonic<H: Hash>(bytes: H) -> String {
    let mut state = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut state);
    mnemonic::to_string(state.finish().to_le_bytes())
}

/// A helper enum to indicate whether a node is in the epoch transition
/// A node is in epoch transition when its high QC is for the last block in an epoch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EpochTransitionIndicator {
    /// A node is currently in the epoch transition
    InTransition,
    /// A node is not in the epoch transition
    NotInTransition,
}

/// Return true if the given block number is the final full block, the "transition block"
#[must_use]
pub fn is_transition_block(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        (block_number + 3) % epoch_height == 0
    }
}
/// returns true if it's the first transition block (epoch height - 2)
#[must_use]
pub fn is_first_transition_block(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        block_number % epoch_height == epoch_height - 2
    }
}
/// Returns true if the block is part of the epoch transition (including the last non null block)  
#[must_use]
pub fn is_epoch_transition(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        block_number % epoch_height >= epoch_height - 3 || block_number % epoch_height == 0
    }
}

/// Returns true if the block is the last block in the epoch
#[must_use]
pub fn is_last_block(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        block_number % epoch_height == 0
    }
}

/// Returns true if the block number is in trasntion but not the transition block
/// or the last block in the epoch.  
///
/// This function is useful for determining if a proposal extending this QC must follow
/// the special rules for transition blocks.
#[must_use]
pub fn is_middle_transition_block(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        let blocks_left = epoch_height - (block_number % epoch_height);
        blocks_left == 1 || blocks_left == 2
    }
}

/// Returns true if the given block number is the third from the last in the epoch based on the
/// given epoch height.
#[must_use]
pub fn is_epoch_root(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        (block_number + 5) % epoch_height == 0
    }
}

/// Returns true if the given block number is equal or greater than the epoch root block
#[must_use]
pub fn is_ge_epoch_root(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        block_number % epoch_height >= epoch_height - 5
    }
}

/// Returns true if the given block number is strictly greater than the epoch root block
pub fn is_gt_epoch_root(block_number: u64, epoch_height: u64) -> bool {
    if block_number == 0 || epoch_height == 0 {
        false
    } else {
        block_number % epoch_height > epoch_height - 5
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_epoch_from_block_number() {
        // block 0 is always epoch 1
        let epoch = epoch_from_block_number(0, 10);
        assert_eq!(1, epoch);

        let epoch = epoch_from_block_number(1, 10);
        assert_eq!(1, epoch);

        let epoch = epoch_from_block_number(10, 10);
        assert_eq!(1, epoch);

        let epoch = epoch_from_block_number(11, 10);
        assert_eq!(2, epoch);

        let epoch = epoch_from_block_number(20, 10);
        assert_eq!(2, epoch);

        let epoch = epoch_from_block_number(21, 10);
        assert_eq!(3, epoch);

        let epoch = epoch_from_block_number(21, 0);
        assert_eq!(0, epoch);
    }

    #[test]
    fn test_is_last_block_in_epoch() {
        assert!(!is_epoch_transition(5, 10));
        assert!(!is_epoch_transition(6, 10));
        assert!(is_epoch_transition(7, 10));
        assert!(is_epoch_transition(8, 10));
        assert!(is_epoch_transition(9, 10));
        assert!(is_epoch_transition(10, 10));
        assert!(!is_epoch_transition(11, 10));

        assert!(!is_epoch_transition(10, 0));
    }

    #[test]
    fn test_is_epoch_root() {
        assert!(is_epoch_root(5, 10));
        assert!(!is_epoch_root(6, 10));
        assert!(!is_epoch_root(7, 10));
        assert!(!is_epoch_root(8, 10));
        assert!(!is_epoch_root(9, 10));
        assert!(!is_epoch_root(10, 10));
        assert!(!is_epoch_root(11, 10));

        assert!(!is_epoch_transition(10, 0));
    }

    #[test]
    fn test_root_block_in_epoch() {
        // block 0 is always epoch 0
        let epoch = 3;
        let epoch_height = 10;
        let epoch_root_block_number = root_block_in_epoch(3, epoch_height);

        assert!(is_epoch_root(25, epoch_height));

        assert_eq!(epoch_root_block_number, 25);

        assert_eq!(
            epoch,
            epoch_from_block_number(epoch_root_block_number, epoch_height)
        );
    }
}
