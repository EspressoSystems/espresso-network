// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{collections::BTreeMap, sync::Arc};

use alloy::transports::BoxFuture;
use anyhow::Context;
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::Leaf2,
    traits::{
        block_contents::BlockHeader, network::ConnectedNetwork, node_implementation::NodeType,
    },
};
use tokio::task::JoinHandle;

use crate::storage_types::TestStorage;

pub struct Leaf2Fetcher<TYPES: NodeType> {
    pub network_functions: NetworkFunctions<TYPES>,
    pub storage: TestStorage<TYPES>,
    pub listener: JoinHandle<()>,
    pub public_key: TYPES::SignatureKey,
}

pub type RecvMessageFn =
    std::sync::Arc<dyn Fn() -> BoxFuture<'static, anyhow::Result<Vec<u8>>> + Send + Sync>;

pub type DirectMessageFn<TYPES> = std::sync::Arc<
    dyn Fn(Vec<u8>, <TYPES as NodeType>::SignatureKey) -> BoxFuture<'static, anyhow::Result<()>>
        + Send
        + Sync,
>;

pub struct NetworkFunctions<TYPES: NodeType> {
    recv_message: RecvMessageFn,
    direct_message: DirectMessageFn<TYPES>,
}

pub async fn recv_message_impl<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
) -> anyhow::Result<Vec<u8>> {
    network
        .recv_message()
        .await
        .context("Failed to receive message from network")
}

pub fn recv_message_fn<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
) -> RecvMessageFn {
    Arc::new(move || {
        let network = network.clone();
        Box::pin(recv_message_impl::<TYPES, I>(network))
    })
}

pub async fn direct_message_impl<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
    message: Vec<u8>,
    recipient: <TYPES as NodeType>::SignatureKey,
) -> anyhow::Result<()> {
    network
        .direct_message(message, recipient.clone())
        .await
        .context(format!("Failed to send message to recipient {}", recipient))
}

pub fn direct_message_fn<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
) -> DirectMessageFn<TYPES> {
    Arc::new(move |message, recipient| {
        let network = network.clone();
        Box::pin(direct_message_impl::<TYPES, I>(network, message, recipient))
    })
}

pub fn network_functions<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
) -> NetworkFunctions<TYPES> {
    let recv_message = recv_message_fn::<TYPES, I>(network.clone());
    let direct_message = direct_message_fn::<TYPES, I>(network.clone());

    NetworkFunctions {
        recv_message,
        direct_message,
    }
}

impl<TYPES: NodeType> Leaf2Fetcher<TYPES> {
    pub fn new<I: NodeImplementation<TYPES>>(
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        storage: TestStorage<TYPES>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        let storage_clone = storage.clone();
        let network_clone = network.clone();
        let listener = tokio::spawn(async move {
//            while let Ok(message) = network_clone.recv_message().await {
//                // Deserialize the message
//                let (requested_height, requester): (u64, TYPES::SignatureKey) =
//                    match bincode::deserialize(&message) {
//                        Ok(message) => message,
//                        Err(e) => {
//                            tracing::error!("Failed to deserialize message: {:?}", e);
//                            continue;
//                        },
//                    };
//
//                let leaves: BTreeMap<u64, Leaf2<TYPES>> = storage_clone
//                    .inner
//                    .read()
//                    .await
//                    .proposals2
//                    .iter()
//                    .map(|(_view, proposal)| {
//                        (
//                            proposal.data.block_header.block_number(),
//                            Leaf2::from_quorum_proposal(&proposal.data.clone().into()),
//                        )
//                    })
//                    .collect();
//
//                let Some(leaf) = leaves.get(&requested_height) else {
//                    tracing::warn!("Block at height {} not found in storage", requested_height);
//                    continue;
//                };
//
//                let serialized_leaf = bincode::serialize(&leaf).expect("Failed to serialized leaf");
//
//                if let Err(e) = network_clone
//                    .direct_message(serialized_leaf, requester)
//                    .await
//                {
//                    tracing::warn!("Failed to send leaf response in test membership fetcher: {e}");
//                };
//            }
        });

        let network_functions: NetworkFunctions<TYPES> = network_functions::<TYPES, I>(network);
        Self {
            network_functions,
            storage,
            listener,
            public_key,
        }
    }

    pub async fn fetch_leaf(
        &self,
        height: u64,
        source: TYPES::SignatureKey,
    ) -> anyhow::Result<Leaf2<TYPES>> {
        let leaf_request = (height, self.public_key.clone());
        let serialized_leaf_request =
            bincode::serialize(&leaf_request).expect("Failed to serialize leaf request");
        if let Err(e) =
            (self.network_functions.direct_message)(serialized_leaf_request, source).await
        {
            tracing::warn!("Failed to send leaf request in test membership fetcher: {e}");
        };

        tokio::time::timeout(std::time::Duration::from_secs(6), async {
            while let Ok(message) = (self.network_functions.recv_message)().await {
                // Deserialize the message
                let leaf: Leaf2<TYPES> = match bincode::deserialize(&message) {
                    Ok(message) => message,
                    Err(e) => {
                        tracing::error!("Failed to deserialize message: {:?}", e);
                        continue;
                    },
                };

                if leaf.height() == height {
                    return Ok(leaf);
                }
            }
            anyhow::bail!("Leaf not found")
        })
        .await
        .context("Leaf fetch timed out")?
    }
}
