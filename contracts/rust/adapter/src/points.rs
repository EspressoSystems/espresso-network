//! Conversions between arkworks curve points and their Solidity representations.

use alloy::primitives::U256;
use ark_ec::{
    AffineRepr,
    short_weierstrass::{Affine, SWCurveConfig},
    twisted_edwards::{self, TECurveConfig},
};
use ark_ff::{Fp2, Fp2Config, PrimeField};

use crate::{
    field_to_u256,
    sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol},
    u256_to_field,
};

impl<P: SWCurveConfig> From<Affine<P>> for G1PointSol
where
    P::BaseField: PrimeField,
{
    fn from(p: Affine<P>) -> Self {
        if p.is_zero() {
            // this convention is from the BN precompile
            Self {
                x: U256::from(0),
                y: U256::from(0),
            }
        } else {
            Self {
                x: field_to_u256::<P::BaseField>(p.x().unwrap()),
                y: field_to_u256::<P::BaseField>(p.y().unwrap()),
            }
        }
    }
}

impl<P: SWCurveConfig> From<G1PointSol> for Affine<P>
where
    P::BaseField: PrimeField,
{
    fn from(p: G1PointSol) -> Self {
        if p == G1PointSol::default() {
            Self::default()
        } else {
            Self::new_unchecked(
                u256_to_field::<P::BaseField>(p.x),
                u256_to_field::<P::BaseField>(p.y),
            )
        }
    }
}

impl<P: SWCurveConfig<BaseField = Fp2<C>>, C> From<G2PointSol> for Affine<P>
where
    C: Fp2Config,
{
    fn from(p: G2PointSol) -> Self {
        Self::new_unchecked(
            Fp2::new(u256_to_field(p.x0), u256_to_field(p.x1)),
            Fp2::new(u256_to_field(p.y0), u256_to_field(p.y1)),
        )
    }
}

impl<P: SWCurveConfig<BaseField = Fp2<C>>, C> From<Affine<P>> for G2PointSol
where
    C: Fp2Config,
{
    fn from(p: Affine<P>) -> Self {
        Self {
            x0: field_to_u256(p.x().unwrap().c0),
            x1: field_to_u256(p.x().unwrap().c1),
            y0: field_to_u256(p.y().unwrap().c0),
            y1: field_to_u256(p.y().unwrap().c1),
        }
    }
}

impl<P: TECurveConfig> From<twisted_edwards::Affine<P>> for EdOnBN254PointSol
where
    P::BaseField: PrimeField,
{
    fn from(p: twisted_edwards::Affine<P>) -> Self {
        Self {
            x: field_to_u256::<P::BaseField>(p.x().unwrap()),
            y: field_to_u256::<P::BaseField>(p.y().unwrap()),
        }
    }
}

impl<P: TECurveConfig> From<EdOnBN254PointSol> for twisted_edwards::Affine<P>
where
    P::BaseField: PrimeField,
{
    fn from(p: EdOnBN254PointSol) -> Self {
        Self::new_unchecked(
            u256_to_field::<P::BaseField>(p.x),
            u256_to_field::<P::BaseField>(p.y),
        )
    }
}
