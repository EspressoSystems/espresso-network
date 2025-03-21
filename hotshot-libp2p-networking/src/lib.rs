// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Library for p2p communication

/// Network logic
pub mod network;

/// symbols needed to implement a networking instance over libp2p-netorking
pub mod reexport {
    pub use libp2p::{request_response::ResponseChannel, Multiaddr};
    pub use libp2p_identity::PeerId;
}
