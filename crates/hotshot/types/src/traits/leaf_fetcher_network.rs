// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network surface used by the test `Leaf2Fetcher` to send catchup
//! request/response messages.  Decoupled from `ConnectedNetwork` so the
//! new-protocol can route through its `Coordinator` instead of holding a
//! second handle to the network.

use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;

use crate::{
    data::ViewNumber,
    traits::{network::ConnectedNetwork, node_implementation::NodeType},
};

/// Operations the test `Leaf2Fetcher` needs to drive epoch catchup.
///
/// Both methods send a serialized payload to a single recipient.  The
/// request/response split is for routing flexibility — implementations may
/// dispatch them differently (e.g. through different queues), even though
/// both legs go over a direct message in the default `ConnectedNetwork`
/// adapter.
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

/// Adapter that satisfies `LeafFetcherNetwork` by sending direct messages
/// over a `ConnectedNetwork` handle.  Used by the old-protocol test infra
/// where the membership owns a clone of the network.
pub struct ConnectedNetworkLeafFetcher<TYPES: NodeType, N> {
    network: Arc<N>,
    _marker: PhantomData<fn() -> TYPES>,
}

impl<TYPES: NodeType, N> ConnectedNetworkLeafFetcher<TYPES, N> {
    pub fn new(network: Arc<N>) -> Self {
        Self {
            network,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<TYPES, N> LeafFetcherNetwork<TYPES> for ConnectedNetworkLeafFetcher<TYPES, N>
where
    TYPES: NodeType,
    N: ConnectedNetwork<TYPES::SignatureKey>,
{
    async fn send_leaf_request(
        &self,
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> anyhow::Result<()> {
        self.network
            .direct_message(view, payload, recipient)
            .await
            .map_err(Into::into)
    }

    async fn send_leaf_response(
        &self,
        view: ViewNumber,
        payload: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> anyhow::Result<()> {
        self.network
            .direct_message(view, payload, recipient)
            .await
            .map_err(Into::into)
    }
}
