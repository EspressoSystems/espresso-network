// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Utilities to help building a stake table.

use alloy::primitives::U256;
use ark_ff::{Field, PrimeField};

/// A trait that converts into a field element.
pub trait ToFields<F: Field> {
    /// The number of field elements needed to represent the given struct.
    const SIZE: usize;

    /// Convert the given struct into a list of field elements.
    fn to_fields(&self) -> Vec<F>;
}

/// convert a U256 to a field element.
pub(crate) fn u256_to_field<F: PrimeField>(v: &U256) -> F {
    let bytes: [u8; 32] = v.to_le_bytes();
    F::from_le_bytes_mod_order(&bytes)
}

#[inline]
/// A helper function to compute the quorum threshold given a total amount of stake.
pub fn one_honest_threshold(total_stake: U256) -> U256 {
    total_stake / U256::from(3) + U256::from(1)
}
