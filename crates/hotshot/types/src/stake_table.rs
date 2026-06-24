// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs related to the stake table

use alloy::primitives::U256;
use ark_ff::PrimeField;
use derive_more::derive::{Deref, DerefMut};
use jf_crhf::CRHF;
use jf_rescue::crhf::VariableLengthRescueCRHF;
use serde::{Deserialize, Serialize};

use crate::{
    NodeType, PeerConfig,
    light_client::{CircuitField, StakeTableState, ToFieldsLightClientCompat},
    traits::signature_key::{SignatureKey, StakeTableEntryType},
};

/// Stake table entry
#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct StakeTableEntry<K: SignatureKey> {
    /// The public key
    pub stake_key: K,
    /// The associated stake amount
    pub stake_amount: U256,
}

impl<K: SignatureKey> StakeTableEntryType<K> for StakeTableEntry<K> {
    /// Get the stake amount
    fn stake(&self) -> U256 {
        self.stake_amount
    }

    /// Get the public key
    fn public_key(&self) -> K {
        self.stake_key.clone()
    }
}

impl<K: SignatureKey> StakeTableEntry<K> {
    /// Get the public key
    pub fn key(&self) -> &K {
        &self.stake_key
    }
}

impl<K: SignatureKey> std::fmt::Debug for StakeTableEntry<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StakeTableEntry")
            .field("stake_key", &format_args!("{}", self.stake_key))
            .field("stake_amount", &self.stake_amount)
            .finish()
    }
}

#[cfg(feature = "rlp")]
mod stake_table_entry_rlp {
    use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};
    use ark_serialize::SerializationError;

    use super::*;

    /// Intermediate type for serializing [`StakeTableEntry`], using [`SignatureKey::to_bytes`] for
    /// the key.
    #[derive(Clone, Debug, RlpDecodable, RlpEncodable)]
    pub(super) struct StakeTableEntryRlp {
        pub(super) stake_key: Vec<u8>,
        pub(super) stake_amount: U256,
    }

    impl<K: SignatureKey> From<&StakeTableEntry<K>> for StakeTableEntryRlp {
        fn from(e: &StakeTableEntry<K>) -> Self {
            Self {
                stake_key: e.stake_key.to_bytes(),
                stake_amount: e.stake_amount,
            }
        }
    }

    impl<K: SignatureKey> TryFrom<StakeTableEntryRlp> for StakeTableEntry<K> {
        type Error = SerializationError;

        fn try_from(e: StakeTableEntryRlp) -> Result<Self, Self::Error> {
            Ok(Self {
                stake_key: K::from_bytes(&e.stake_key)?,
                stake_amount: e.stake_amount,
            })
        }
    }

    impl<K: SignatureKey> Encodable for StakeTableEntry<K> {
        fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
            StakeTableEntryRlp::from(self).encode(out)
        }

        fn length(&self) -> usize {
            StakeTableEntryRlp::from(self).length()
        }
    }

    impl<K: SignatureKey> Decodable for StakeTableEntry<K> {
        fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
            let rlp = StakeTableEntryRlp::decode(buf)?;
            rlp.try_into().map_err(|err| {
                tracing::warn!("malformed StakeTableEntry: {err:#}");
                alloy_rlp::Error::Custom("input is valid RLP but not a valid StakeTableEntry")
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Deref, DerefMut)]
pub struct HSStakeTable<TYPES: NodeType>(pub Vec<PeerConfig<TYPES>>);

impl<TYPES: NodeType> From<Vec<PeerConfig<TYPES>>> for HSStakeTable<TYPES> {
    fn from(peers: Vec<PeerConfig<TYPES>>) -> Self {
        Self(peers)
    }
}

impl<'a, T: NodeType> FromIterator<&'a PeerConfig<T>> for HSStakeTable<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a PeerConfig<T>>,
    {
        Self(iter.into_iter().cloned().collect())
    }
}

/// A helper function to compute the quorum threshold given a total amount of stake.
#[inline]
pub fn one_honest_threshold(total_stake: U256) -> U256 {
    total_stake / U256::from(3) + U256::from(1)
}

#[inline]
/// A helper function to compute the fault tolerant quorum threshold given a total amount of stake.
pub fn supermajority_threshold(total_stake: U256) -> U256 {
    let one = U256::ONE;
    let two = U256::from(2);
    let three = U256::from(3);
    if total_stake < U256::MAX / two {
        ((total_stake * two) / three) + one
    } else {
        ((total_stake / three) * two) + two
    }
}

#[inline]
fn u256_to_field(amount: U256) -> CircuitField {
    let amount_bytes: [u8; 32] = amount.to_le_bytes();
    CircuitField::from_le_bytes_mod_order(&amount_bytes)
}

impl<TYPES: NodeType> std::iter::IntoIterator for HSStakeTable<TYPES> {
    type Item = PeerConfig<TYPES>;
    type IntoIter = std::vec::IntoIter<PeerConfig<TYPES>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<TYPES: NodeType> HSStakeTable<TYPES> {
    pub fn commitment(&self, stake_table_capacity: usize) -> anyhow::Result<StakeTableState> {
        if stake_table_capacity < self.0.len() {
            return Err(anyhow::anyhow!(
                "Stake table over capacity: {} < {}",
                stake_table_capacity,
                self.0.len(),
            ));
        }
        let padding_len = stake_table_capacity - self.0.len();
        let mut bls_preimage = vec![];
        let mut schnorr_preimage = vec![];
        let mut amount_preimage = vec![];
        let mut total_stake = U256::from(0);
        for peer in &self.0 {
            bls_preimage.extend(peer.stake_table_entry.public_key().to_fields());
            schnorr_preimage.extend(peer.state_ver_key.to_fields());
            amount_preimage.push(u256_to_field(peer.stake_table_entry.stake()));
            total_stake += peer.stake_table_entry.stake();
        }
        bls_preimage.resize(
            <TYPES::SignatureKey as ToFieldsLightClientCompat>::SIZE * stake_table_capacity,
            CircuitField::default(),
        );
        // Nasty tech debt
        schnorr_preimage.extend(
            std::iter::repeat_n(TYPES::StateSignatureKey::default().to_fields(), padding_len)
                .flatten(),
        );
        amount_preimage.resize(stake_table_capacity, CircuitField::default());
        let threshold = u256_to_field(one_honest_threshold(total_stake));
        Ok(StakeTableState {
            bls_key_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(bls_preimage)
                .unwrap()[0],
            schnorr_key_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(
                schnorr_preimage,
            )
            .unwrap()[0],
            amount_comm: VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(amount_preimage)
                .unwrap()[0],
            threshold,
        })
    }

    pub fn total_stakes(&self) -> U256 {
        self.0
            .iter()
            .map(|peer| peer.stake_table_entry.stake())
            .sum()
    }
}

pub struct StakeTableEntries<TYPES: NodeType>(
    pub Vec<<<TYPES as NodeType>::SignatureKey as SignatureKey>::StakeTableEntry>,
);

impl<TYPES: NodeType> From<Vec<PeerConfig<TYPES>>> for StakeTableEntries<TYPES> {
    fn from(peers: Vec<PeerConfig<TYPES>>) -> Self {
        Self(
            peers
                .into_iter()
                .map(|peer| peer.stake_table_entry)
                .collect::<Vec<_>>(),
        )
    }
}

impl<TYPES: NodeType> From<HSStakeTable<TYPES>> for StakeTableEntries<TYPES> {
    fn from(stake_table: HSStakeTable<TYPES>) -> Self {
        Self::from(stake_table.0)
    }
}

impl<'a, T: NodeType> FromIterator<&'a PeerConfig<T>> for StakeTableEntries<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a PeerConfig<T>>,
    {
        Self(
            iter.into_iter()
                .map(|peer| peer.stake_table_entry.clone())
                .collect(),
        )
    }
}

#[cfg(all(test, feature = "rlp"))]
mod rlp_test {
    use alloy_rlp::{Decodable, Encodable};
    use zeroize::Zeroize;

    use super::*;
    use crate::{signature_key::BLSPubKey, stake_table::stake_table_entry_rlp::StakeTableEntryRlp};

    #[test_log::test]
    fn bls_stake_table_entry_rlp_round_trip_random() {
        let entry = StakeTableEntry {
            stake_key: BLSPubKey::generated_from_seed_indexed(
                Default::default(),
                Default::default(),
            )
            .0,
            stake_amount: U256::ONE,
        };

        let mut bytes = vec![];
        entry.encode(&mut bytes);
        assert_eq!(bytes.len(), entry.length());

        let mut buf = bytes.as_slice();
        assert_eq!(entry, StakeTableEntry::decode(&mut buf).unwrap());
        assert!(buf.is_empty());
    }

    #[test_log::test]
    fn bls_stake_table_entry_rlp_round_trip_zero() {
        let mut stake_key =
            BLSPubKey::generated_from_seed_indexed(Default::default(), Default::default()).0;
        stake_key.zeroize();
        let entry = StakeTableEntry {
            stake_key,
            stake_amount: U256::ZERO,
        };

        let mut bytes = vec![];
        entry.encode(&mut bytes);
        assert_eq!(bytes.len(), entry.length());

        let mut buf = bytes.as_slice();
        assert_eq!(entry, StakeTableEntry::decode(&mut buf).unwrap());
        assert!(buf.is_empty());
    }

    #[test_log::test]
    fn bls_stake_table_entry_invalid_malformed_key() {
        let entry = StakeTableEntryRlp {
            stake_key: "not a key".as_bytes().to_vec(),
            stake_amount: U256::ZERO,
        };

        let mut buf = vec![];
        entry.encode(&mut buf);
        let err = StakeTableEntry::<BLSPubKey>::decode(&mut buf.as_slice()).unwrap_err();
        assert_eq!(
            err,
            alloy_rlp::Error::Custom("input is valid RLP but not a valid StakeTableEntry"),
        );
    }
}
