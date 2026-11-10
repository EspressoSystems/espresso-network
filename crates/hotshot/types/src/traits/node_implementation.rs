// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{fmt::Debug, hash::Hash};

use serde::Deserialize;

use super::{
    ValidatedState,
    block_contents::{BlockHeader, Transaction},
    signature_key::{
        BuilderSignatureKey, LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey,
        StateSignatureKey,
    },
};
use crate::traits::{
    BlockPayload, election::Membership, signature_key::SignatureKey, states::InstanceState,
};

/// Trait with all the type definitions that are used in the current hotshot setup.
pub trait NodeType:
    Clone
    + Copy
    + Debug
    + Hash
    + PartialEq
    + Eq
    + Default
    + serde::Serialize
    + for<'de> Deserialize<'de>
    + Send
    + Sync
    + 'static
{
    /// The block header type that this hotshot setup is using.
    type BlockHeader: BlockHeader<Self>;
    /// The block type that this hotshot setup is using.
    ///
    /// This should be the same block that `ValidatedState::BlockPayload` is using.
    type BlockPayload: BlockPayload<
            Self,
            Instance = Self::InstanceState,
            Transaction = Self::Transaction,
            ValidatedState = Self::ValidatedState,
        >;
    /// The signature key that this hotshot setup is using.
    type SignatureKey: SignatureKey;
    /// The transaction type that this hotshot setup is using.
    ///
    /// This should be equal to `BlockPayload::Transaction`
    type Transaction: Transaction;

    /// The instance-level state type that this hotshot setup is using.
    type InstanceState: InstanceState;

    /// The validated state type that this hotshot setup is using.
    type ValidatedState: ValidatedState<Self, Instance = Self::InstanceState>;

    /// Membership used for this implementation
    type Membership: Membership<Self>;

    /// The type builder uses to sign its messages
    type BuilderSignatureKey: BuilderSignatureKey;

    /// The type replica uses to sign the light client state
    type StateSignatureKey: StateSignatureKey
        + LCV1StateSignatureKey
        + LCV2StateSignatureKey
        + LCV3StateSignatureKey;
}
