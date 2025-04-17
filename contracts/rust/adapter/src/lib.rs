//! Cross-domain (between Solidity and Rust) utilities for type conversion and testing

use alloy::primitives::U256;
use ark_ff::{BigInteger, PrimeField};

#[allow(dead_code)]
pub(crate) mod bindings;
pub mod evm;
pub mod jellyfish;
pub mod light_client;
pub mod sol_types;
pub mod stake_table;

/// convert a field element to U256, panic if field size is larger than 256 bit
pub fn field_to_u256<F: PrimeField>(f: F) -> U256 {
    if F::MODULUS_BIT_SIZE > 256 {
        panic!("Shouldn't convert a >256-bit field to U256");
    }
    U256::from_le_slice(&f.into_bigint().to_bytes_le())
}

/// convert U256 to a field (mod order)
pub fn u256_to_field<F: PrimeField>(x: U256) -> F {
    let bytes: [u8; 32] = x.to_le_bytes();
    F::from_le_bytes_mod_order(&bytes)
}
