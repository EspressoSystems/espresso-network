// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network surface used by the test `Leaf2Fetcher` to send catchup
//! request/response messages.  The new-protocol routes it through its
//! `Coordinator` instead of holding a second handle to the network.

use async_trait::async_trait;

use crate::{data::ViewNumber, traits::node_implementation::NodeType};

/// Operations the test `Leaf2Fetcher` needs to drive epoch catchup.
///
/// Both methods send a serialized payload to a single recipient.  The
/// request/response split is for routing flexibility — implementations may
/// dispatch them differently (e.g. through different queues).
#[async_trait]
pub trait LeafFetcherNetwork<TYPES: NodeType>: Send + Sync + 'static {
    async fn send_leaf_request(
        &self,
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> anyhow::Result<()>;

    async fn send_leaf_response(
        &self,
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> anyhow::Result<()>;
}
