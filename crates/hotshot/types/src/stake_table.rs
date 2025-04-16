// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs related to the stake table

use alloy::primitives::U256;
use serde::{Deserialize, Serialize};

use crate::traits::signature_key::{SignatureKey, StakeTableEntryType};

/// Stake table entry
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash, Eq)]
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

// TODO(Chengyu): add stake table snapshot here
