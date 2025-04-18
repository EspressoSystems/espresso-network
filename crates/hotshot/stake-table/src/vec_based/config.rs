// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Config file for stake table
use ark_ff::PrimeField;
use ark_std::vec;
/// Schnorr verification key as auxiliary information
pub use hotshot_types::light_client::StateVerKey;
/// BLS verification key as indexing key
pub use jf_signature::bls_over_bn254::VerKey as QCVerKey;
use jf_utils::to_bytes;

use crate::utils::ToFields;
/// Type for commitment
pub type FieldType = ark_ed_on_bn254::Fq;

/// Hashable representation of a key
/// NOTE: commitment is only used in light client contract.
/// For this application, we needs only hash the Schnorr verification key.
impl ToFields<FieldType> for StateVerKey {
    const SIZE: usize = 2;

    fn to_fields(&self) -> Vec<FieldType> {
        let p = self.to_affine();
        vec![p.x, p.y]
    }
}

impl ToFields<FieldType> for QCVerKey {
    const SIZE: usize = 3;

    fn to_fields(&self) -> Vec<FieldType> {
        #[allow(clippy::ignored_unit_patterns)]
        match to_bytes!(&self.to_affine()) {
            Ok(bytes) => {
                vec![
                    FieldType::from_le_bytes_mod_order(&bytes[..31]),
                    FieldType::from_le_bytes_mod_order(&bytes[31..62]),
                    FieldType::from_le_bytes_mod_order(&bytes[62..]),
                ]
            },
            Err(_) => unreachable!(),
        }
    }
}
