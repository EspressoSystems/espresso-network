// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Contains the [`ConsensusApi`] trait.

use std::{num::NonZeroUsize, time::Duration};

use async_trait::async_trait;

use crate::{
    event::Event,
    traits::{
        node_implementation::{NodeImplementation, NodeType},
        signature_key::{SignatureKey, StateSignatureKey},
    },
};

/// The API that tasks use to talk to the system
/// TODO we plan to drop this <https://github.com/EspressoSystems/HotShot/issues/2294>
#[async_trait]
pub trait ConsensusApi<TYPES: NodeType, I: NodeImplementation<TYPES>>: Send + Sync {
    /// Total number of nodes in the network. Also known as `n`.
    fn total_nodes(&self) -> NonZeroUsize;

    /// The maximum amount of time a leader can wait to get a block from a builder.
    fn builder_timeout(&self) -> Duration;

    /// Get a reference to the public key.
    fn public_key(&self) -> &TYPES::SignatureKey;

    /// Get a reference to the private key.
    fn private_key(&self) -> &<TYPES::SignatureKey as SignatureKey>::PrivateKey;

    /// Get a reference to the light client signing key.
    fn state_private_key(
        &self,
    ) -> &<TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey;

    /// Notify the system of an event within `hotshot-consensus`.
    async fn send_event(&self, event: Event<TYPES>);
}
