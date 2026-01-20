// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

/// Sortition trait
mod networking;
mod node_implementation;

pub use hotshot_libp2p_networking::network::NetworkNodeConfigBuilder;
pub use hotshot_types::traits::{BlockPayload, ValidatedState};
pub use networking::{NetworkError, NetworkReliability};
pub use node_implementation::{NodeImplementation, TestableNodeImplementation};

/// Module for publicly usable implementations of the traits
pub mod implementations {
    pub use super::networking::{
        cliquenet_network::{
            derive_keypair as derive_cliquenet_keypair, Address as CliquenetAddress, Cliquenet,
            PublicKey as CliquenetPublicKey,
        },
        combined_network::{CombinedNetworks, UnderlyingCombinedNetworks},
        memory_network::{MasterMap, MemoryNetwork},
        push_cdn_network::{
            CdnMetricsValue, KeyPair, ProductionDef, PushCdnNetwork, TestingDef, Topic as CdnTopic,
            WrappedSignatureKey,
        },
    };
}
