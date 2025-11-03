// Copyright (c) 2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot-VID library.

// You should have received a copy of the MIT License
// along with the HotShot-VID library. If not, see <https://mit-license.org/>.

//! Verifiable Information Retrieval (VID).
#![deny(missing_docs)]

use displaydoc::Display;
use jf_merkle_tree::MerkleTreeError;
use jf_poseidon2::Poseidon2Error;
use serde::{Deserialize, Serialize};

pub mod avidm;
pub mod avidm_gf2;
mod utils;

/// A glorified [`bool`] that leverages compile lints to encourage the caller to
/// use the result.
///
/// Intended as the return type for verification of proofs, signatures, etc.
/// Recommended for use in the nested [`Result`] pattern: see <https://sled.rs/errors>.
type VerificationResult = Result<(), ()>;

/// The error type for `VidScheme` methods.
#[derive(Display, Debug)]
pub enum VidError {
    /// invalid args: {0}
    Argument(String),
    /// internal error: {0}
    Internal(anyhow::Error),
    /// Insufficient shares
    InsufficientShares,
    /// Share index out of bound
    IndexOutOfBound,
    /// Invalid parameter
    InvalidParam,
    /// Invalid VID share
    InvalidShare,
}

impl From<Poseidon2Error> for VidError {
    fn from(err: Poseidon2Error) -> Self {
        VidError::Internal(err.into())
    }
}

impl From<MerkleTreeError> for VidError {
    fn from(err: MerkleTreeError) -> Self {
        VidError::Internal(err.into())
    }
}

impl From<reed_solomon_simd::Error> for VidError {
    fn from(err: reed_solomon_simd::Error) -> Self {
        VidError::Internal(err.into())
    }
}

/// Alias
type VidResult<T> = Result<T, VidError>;

/// Trait definition for a Verifiable Information Dispersal (VID) scheme.
pub trait VidScheme {
    /// VID Parameters
    type Param: Send + Sync + Serialize + for<'a> Deserialize<'a>;

    /// VID Share type
    type Share: Send + Sync + Serialize + for<'a> Deserialize<'a>;

    /// VID commitment type
    type Commit: Eq + PartialEq + Send + Sync + Serialize + for<'a> Deserialize<'a>;

    /// Commit to a `payload` without generating shares.
    fn commit(param: &Self::Param, payload: &[u8]) -> VidResult<Self::Commit>;

    /// Disperse the given `payload` according to the weights in `distribution`.
    fn disperse(
        param: &Self::Param,
        distribution: &[u32],
        payload: &[u8],
    ) -> VidResult<(Self::Commit, Vec<Self::Share>)>;

    /// Verify the given VID `share` against the VID `commit`.
    #[allow(clippy::result_unit_err)]
    fn verify_share(
        param: &Self::Param,
        commit: &Self::Commit,
        share: &Self::Share,
    ) -> VidResult<VerificationResult>;

    /// Recover the payload from the given `shares`.
    fn recover(
        param: &Self::Param,
        commit: &Self::Commit,
        shares: &[Self::Share],
    ) -> VidResult<Vec<u8>>;
}
