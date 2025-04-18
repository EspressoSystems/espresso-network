// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the implementation for ADVZ VID scheme

use std::{fmt::Debug, ops::Range};

use ark_bn254::Bn254;
use jf_pcs::{
    prelude::{UnivariateKzgPCS, UnivariateUniversalParams},
    PolynomialCommitmentScheme,
};
use jf_vid::{
    advz::{
        self,
        payload_prover::{LargeRangeProof, SmallRangeProof},
    },
    payload_prover::{PayloadProver, Statement},
    VidDisperse, VidResult, VidScheme,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::constants::SRS_DEGREE;

/// VID scheme constructor.
///
/// Returns an opaque type that impls jellyfish traits:
/// [`VidScheme`], [`PayloadProver`], [`Precomputable`].
///
/// # Rust forbids naming impl Trait in return types
///
/// Due to Rust limitations the return type of [`vid_scheme`] is a newtype
/// wrapper [`VidSchemeType`] that impls the above traits.
///
/// We prefer that the return type of [`vid_scheme`] be `impl Trait` for the
/// above traits. But the ability to name an impl Trait return type is
/// currently missing from Rust:
/// - [Naming impl trait in return types - Impl trait initiative](https://rust-lang.github.io/impl-trait-initiative/explainer/rpit_names.html)
/// - [RFC: Type alias impl trait (TAIT)](https://github.com/rust-lang/rfcs/blob/master/text/2515-type_alias_impl_trait.md)
///
/// # Panics
/// When the construction fails for the underlying VID scheme.
#[must_use]
#[memoize::memoize(SharedCache, Capacity: 10)]
pub fn advz_scheme(num_storage_nodes: usize) -> ADVZScheme {
    // recovery_threshold is currently num_storage_nodes rounded down to a power of two
    // TODO recovery_threshold should be a function of the desired erasure code rate
    // https://github.com/EspressoSystems/HotShot/issues/2152
    let recovery_threshold = 1 << num_storage_nodes.ilog2();

    #[allow(clippy::panic)]
    let num_storage_nodes = u32::try_from(num_storage_nodes).unwrap_or_else(|err| {
        panic!(
            "num_storage_nodes {num_storage_nodes} should fit into u32; \
                error: {err}"
        )
    });

    // TODO panic, return `Result`, or make `new` infallible upstream (eg. by panicking)?
    #[allow(clippy::panic)]
    ADVZScheme(
        Advz::new(num_storage_nodes, recovery_threshold, &*KZG_SRS).unwrap_or_else(|err| {
              panic!("advz construction failure: (num_storage nodes,recovery_threshold)=({num_storage_nodes},{recovery_threshold}); \
                      error: {err}")
        })
    )
}

/// VID commitment type
pub type ADVZCommitment = <ADVZScheme as VidScheme>::Commit;
/// VID common type
pub type ADVZCommon = <ADVZScheme as VidScheme>::Common;
/// VID share type
pub type ADVZShare = <ADVZScheme as VidScheme>::Share;

#[cfg(not(feature = "gpu-vid"))]
/// Internal Jellyfish VID scheme
type Advz = advz::Advz<E, H>;
#[cfg(feature = "gpu-vid")]
/// Internal Jellyfish VID scheme
type Advz = advz::AdvzGPU<'static, E, H>;

/// Newtype wrapper for a VID scheme type that impls
/// [`VidScheme`], [`PayloadProver`], [`Precomputable`].
#[derive(Clone)]
pub struct ADVZScheme(Advz);

/// Newtype wrapper for a large payload range proof.
///
/// Useful for namespace proofs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LargeRangeProofType(
    // # Type complexity
    //
    // Jellyfish's `LargeRangeProof` type has a prime field generic parameter `F`.
    // This `F` is determined by the type parameter `E` for `Advz`.
    // Jellyfish needs a more ergonomic way for downstream users to refer to this type.
    //
    // There is a `KzgEval` type alias in jellyfish that helps a little, but it's currently private:
    // <https://github.com/EspressoSystems/jellyfish/issues/423>
    // If it were public then we could instead use
    // `LargeRangeProof<KzgEval<E>>`
    // but that's still pretty crufty.
    LargeRangeProof<<UnivariateKzgPCS<E> as PolynomialCommitmentScheme>::Evaluation>,
);

/// Newtype wrapper for a small payload range proof.
///
/// Useful for transaction proofs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SmallRangeProofType(
    // # Type complexity
    //
    // Similar to the comments in `LargeRangeProofType`.
    SmallRangeProof<<UnivariateKzgPCS<E> as PolynomialCommitmentScheme>::Proof>,
);

// By default, use SRS from Aztec's ceremony
lazy_static! {
    /// SRS comment
    static ref KZG_SRS: UnivariateUniversalParams<E> = {
        let srs = ark_srs::kzg10::aztec20::setup(SRS_DEGREE)
            .expect("Aztec SRS failed to load");
        UnivariateUniversalParams {
            powers_of_g: srs.powers_of_g,
            h: srs.h,
            beta_h: srs.beta_h,
            powers_of_h: vec![srs.h, srs.beta_h],
        }
    };
}

/// Private type alias for the EC pairing type parameter for [`Advz`].
type E = Bn254;
/// Private type alias for the hash type parameter for [`Advz`].
type H = Sha256;

// THE REST OF THIS FILE IS BOILERPLATE
//
// All this boilerplate can be deleted when we finally get
// type alias impl trait (TAIT):
// [rfcs/text/2515-type_alias_impl_trait.md at master · rust-lang/rfcs](https://github.com/rust-lang/rfcs/blob/master/text/2515-type_alias_impl_trait.md)
impl VidScheme for ADVZScheme {
    type Commit = <Advz as VidScheme>::Commit;
    type Share = <Advz as VidScheme>::Share;
    type Common = <Advz as VidScheme>::Common;

    fn commit_only<B>(&mut self, payload: B) -> VidResult<Self::Commit>
    where
        B: AsRef<[u8]>,
    {
        self.0.commit_only(payload)
    }

    fn disperse<B>(&mut self, payload: B) -> VidResult<VidDisperse<Self>>
    where
        B: AsRef<[u8]>,
    {
        self.0.disperse(payload).map(vid_disperse_conversion)
    }

    fn verify_share(
        &self,
        share: &Self::Share,
        common: &Self::Common,
        commit: &Self::Commit,
    ) -> VidResult<Result<(), ()>> {
        self.0.verify_share(share, common, commit)
    }

    fn recover_payload(&self, shares: &[Self::Share], common: &Self::Common) -> VidResult<Vec<u8>> {
        self.0.recover_payload(shares, common)
    }

    fn is_consistent(commit: &Self::Commit, common: &Self::Common) -> VidResult<()> {
        <Advz as VidScheme>::is_consistent(commit, common)
    }

    fn get_payload_byte_len(common: &Self::Common) -> u32 {
        <Advz as VidScheme>::get_payload_byte_len(common)
    }

    fn get_num_storage_nodes(common: &Self::Common) -> u32 {
        <Advz as VidScheme>::get_num_storage_nodes(common)
    }

    fn get_multiplicity(common: &Self::Common) -> u32 {
        <Advz as VidScheme>::get_multiplicity(common)
    }
}

impl PayloadProver<LargeRangeProofType> for ADVZScheme {
    fn payload_proof<B>(&self, payload: B, range: Range<usize>) -> VidResult<LargeRangeProofType>
    where
        B: AsRef<[u8]>,
    {
        self.0
            .payload_proof(payload, range)
            .map(LargeRangeProofType)
    }

    fn payload_verify(
        &self,
        stmt: Statement<'_, Self>,
        proof: &LargeRangeProofType,
    ) -> VidResult<Result<(), ()>> {
        self.0.payload_verify(stmt_conversion(stmt), &proof.0)
    }
}

impl PayloadProver<SmallRangeProofType> for ADVZScheme {
    fn payload_proof<B>(&self, payload: B, range: Range<usize>) -> VidResult<SmallRangeProofType>
    where
        B: AsRef<[u8]>,
    {
        self.0
            .payload_proof(payload, range)
            .map(SmallRangeProofType)
    }

    fn payload_verify(
        &self,
        stmt: Statement<'_, Self>,
        proof: &SmallRangeProofType,
    ) -> VidResult<Result<(), ()>> {
        self.0.payload_verify(stmt_conversion(stmt), &proof.0)
    }
}

/// Convert a [`VidDisperse<Advz>`] to a [`VidDisperse<VidSchemeType>`].
///
/// Foreign type rules prevent us from doing:
/// - `impl From<VidDisperse<VidSchemeType>> for VidDisperse<Advz>`
/// - `impl VidDisperse<VidSchemeType> {...}`
///
/// and similarly for `Statement`.
/// Thus, we accomplish type conversion via functions.
fn vid_disperse_conversion(vid_disperse: VidDisperse<Advz>) -> VidDisperse<ADVZScheme> {
    VidDisperse {
        shares: vid_disperse.shares,
        common: vid_disperse.common,
        commit: vid_disperse.commit,
    }
}

/// Convert a [`Statement<'_, VidSchemeType>`] to a [`Statement<'_, Advz>`].
fn stmt_conversion(stmt: Statement<'_, ADVZScheme>) -> Statement<'_, Advz> {
    Statement {
        payload_subslice: stmt.payload_subslice,
        range: stmt.range,
        commit: stmt.commit,
        common: stmt.common,
    }
}
