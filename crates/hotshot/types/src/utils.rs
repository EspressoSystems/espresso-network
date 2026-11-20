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

use anyhow::{anyhow, ensure};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bincode::{
    DefaultOptions, Options,
    config::{
        FixintEncoding, LittleEndian, RejectTrailing, WithOtherEndian, WithOtherIntEncoding,
        WithOtherLimit, WithOtherTrailing,
    },
};
use committable::{Commitment, Committable};
use digest::OutputSizeUser;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tagged_base64::tagged;
use typenum::Unsigned;
use vbs::version::Version;
use versions::EPOCH_VERSION;

use crate::{
    data::{EpochNumber, Leaf2, VidCommitment},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::QuorumCertificate2,
    simple_vote::HasEpoch,
    stake_table::StakeTableEntries,
    traits::{ValidatedState, election::Membership, node_implementation::NodeType},
    vote::{Certificate, HasViewNumber},
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
        epoch: Option<EpochNumber>,
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
        epoch: Option<EpochNumber>,
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

pub async fn verify_leaf_chain<T: NodeType>(
    mut leaf_chain: Vec<Leaf2<T>>,
    coordinator: &EpochMembershipCoordinator<T>,
    expected_height: u64,
    upgrade_lock: &UpgradeLock<T>,
) -> anyhow::Result<Leaf2<T>> {
    let epoch_length = coordinator.epoch_length();
    let first_epoch = coordinator.membership().first_epoch();
    // Derive each QC's epoch from the height of the leaf it certifies; trusting
    // the epoch claimed in the QC would let a stale epoch's quorum pick the
    // stake table that verifies its own signatures.
    let validate_qc = |qc: &QuorumCertificate2<T>, certified_height: u64| -> anyhow::Result<()> {
        let epoch = epoch_length.epoch_of(certified_height);
        let (entries, threshold) = match qc.data.epoch() {
            // QCs formed before epochs were enabled carry no epoch. Epochs start part way
            // through the first epoch, which runs on the genesis stake table like the
            // non-epoch chain before it, so no later height may be certified this way.
            None => {
                ensure!(
                    epoch <= first_epoch,
                    "QC claims no epoch but certifies the leaf at height {certified_height} in \
                     epoch {epoch}, after the first epoch {first_epoch}"
                );
                let genesis = coordinator.membership().genesis_stake_tables();
                (
                    StakeTableEntries::<T>::from_iter(genesis.stake_table()).0,
                    genesis.success_threshold(),
                )
            },
            Some(claimed) => {
                ensure!(
                    claimed == epoch,
                    "QC claims epoch {claimed} but certifies the leaf at height \
                     {certified_height} in epoch {epoch}"
                );
                let membership = coordinator.stake_table_for_epoch(epoch).map_err(|err| {
                    anyhow!("no stake table available for epoch {epoch}: {err:?}")
                })?;
                (
                    StakeTableEntries::<T>::from_iter(membership.stake_table()).0,
                    membership.success_threshold(),
                )
            },
        };
        if let Some(block_number) = qc.data.block_number {
            ensure!(
                block_number == certified_height,
                "QC claims block number {block_number} but certifies the leaf at height \
                 {certified_height}"
            );
        }
        qc.is_valid_cert(&entries, threshold, upgrade_lock)?;
        Ok(())
    };

    // Sort the leaf chain by view number
    leaf_chain.sort_by_key(|l| l.view_number());
    // Reverse it
    leaf_chain.reverse();

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

    // Verify the root is in the chain of decided leaves
    validate_qc(newest_leaf.justify_qc(), parent.height())?;
    let mut last_leaf = parent;
    for leaf in leaf_chain.iter().skip(2) {
        ensure!(last_leaf.justify_qc().view_number() == leaf.view_number());
        ensure!(last_leaf.justify_qc().data().leaf_commit == leaf.commit());
        validate_qc(last_leaf.justify_qc(), leaf.height())?;
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
    pub fn epoch(&self) -> Option<Option<EpochNumber>> {
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

    /// The commitment to no bytes, for where one is required but nothing was
    /// committed to. Not the all-zero digest `Default` gives.
    pub fn empty() -> Self {
        Self::from_bytes([])
    }
}

impl AsRef<Sha256Digest> for BuilderCommitment {
    fn as_ref(&self) -> &Sha256Digest {
        &self.0
    }
}

type BincodeOpts = WithOtherTrailing<
    WithOtherIntEncoding<
        WithOtherEndian<WithOtherLimit<DefaultOptions, bincode::config::Infinite>, LittleEndian>,
        FixintEncoding,
    >,
    RejectTrailing,
>;

/// For the wire format, we use bincode with the following options:
///   - No upper size limit
///   - Little endian encoding
///   - Varint encoding
///   - Reject trailing bytes
#[must_use]
pub fn bincode_opts() -> BincodeOpts {
    bincode::DefaultOptions::new()
        .with_no_limit()
        .with_little_endian()
        .with_fixint_encoding()
        .reject_trailing_bytes()
}

/// Returns Some(1) if epochs are enabled by `base`, otherwise returns None
#[must_use]
pub fn genesis_epoch_from_version(base: Version) -> Option<EpochNumber> {
    (base >= EPOCH_VERSION).then(|| EpochNumber::new(1))
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
