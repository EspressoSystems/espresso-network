// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{collections::BTreeMap, sync::Arc};

use alloy::transports::BoxFuture;
use anyhow::Context;
use async_broadcast::{Receiver, RecvError};
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::Leaf2,
    event::{Event, EventType},
    message::{Message, MessageKind},
    traits::{
        block_contents::BlockHeader, network::ConnectedNetwork, node_implementation::NodeType,
    },
};
use tokio::task::JoinHandle;
use vbs::{bincode_serializer::BincodeSerializer, version::StaticVersion, BinarySerializer};

use crate::storage_types::TestStorage;

pub struct Leaf2Fetcher<TYPES: NodeType> {
    pub network_functions: NetworkFunctions<TYPES>,
    pub storage: TestStorage<TYPES>,
    pub listener: Option<JoinHandle<()>>,
    pub public_key: TYPES::SignatureKey,
    pub network_receiver: Option<Receiver<Event<TYPES>>>,
}

pub type RecvMessageFn =
    std::sync::Arc<dyn Fn() -> BoxFuture<'static, anyhow::Result<Vec<u8>>> + Send + Sync>;

pub type DirectMessageFn<TYPES> = std::sync::Arc<
    dyn Fn(Vec<u8>, <TYPES as NodeType>::SignatureKey) -> BoxFuture<'static, anyhow::Result<()>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct NetworkFunctions<TYPES: NodeType> {
    direct_message: DirectMessageFn<TYPES>,
}

pub async fn direct_message_impl<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
    message: Vec<u8>,
    recipient: <TYPES as NodeType>::SignatureKey,
) -> anyhow::Result<()> {
    network
        .direct_message(message, recipient.clone())
        .await
        .context(format!("Failed to send message to recipient {recipient}"))
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
    let direct_message = direct_message_fn::<TYPES, I>(network.clone());

    NetworkFunctions { direct_message }
}

impl<TYPES: NodeType> Leaf2Fetcher<TYPES> {
    pub fn new<I: NodeImplementation<TYPES>>(
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        storage: TestStorage<TYPES>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        let listener = None;

        let network_functions: NetworkFunctions<TYPES> = network_functions::<TYPES, I>(network);
        Self {
            network_functions,
            storage,
            listener,
            public_key,
            network_receiver: None,
        }
    }

    pub fn set_external_channel(&mut self, mut network_receiver: Receiver<Event<TYPES>>) {
        let public_key = self.public_key.clone();
        let storage = self.storage.clone();
        let network_functions = self.network_functions.clone();

        self.network_receiver = Some(network_receiver.clone());

        let listener = tokio::spawn(async move {
            loop {
                match network_receiver.recv_direct().await {
                    Ok(Event {
                        view_number: _,
                        event: EventType::ExternalMessageReceived { sender: _, data },
                    }) => {
                        let (requested_height, requester): (u64, TYPES::SignatureKey) =
                            match bincode::deserialize(&data) {
                                Ok(message) => message,
                                Err(e) => {
                                    tracing::debug!("Failed to deserialize message: {e:?}");
                                    continue;
                                },
                            };

                        let leaves: BTreeMap<u64, Leaf2<TYPES>> = storage
                            .inner
                            .read()
                            .await
                            .proposals_wrapper
                            .values()
                            .map(|proposal| {
                                (
                                    proposal.data.block_header().block_number(),
                                    Leaf2::from_quorum_proposal(&proposal.data.clone()),
                                )
                            })
                            .collect();

                        let heights = leaves.keys().collect::<Vec<_>>();

                        let Some(leaf) = leaves.get(&requested_height) else {
                            tracing::error!(
                                "Block at height {requested_height} not found in storage.\n\n \
                                 stored leaf heights: {heights:?}"
                            );
                            continue;
                        };

                        let leaf_response = Message {
                            sender: public_key.clone(),
                            kind: MessageKind::<TYPES>::External(
                                bincode::serialize(&leaf).expect("Failed to serialize leaf"),
                            ),
                        };

                        let serialized_leaf_response =
                            BincodeSerializer::<StaticVersion<0, 0>>::serialize(&leaf_response)
                                .expect("Failed to serialize leaf response");

                        if let Err(e) =
                            (network_functions.direct_message)(serialized_leaf_response, requester)
                                .await
                        {
                            tracing::error!(
                                "Failed to send leaf response in test membership fetcher: {e}"
                            );
                        };
                    },
                    Err(RecvError::Closed) => {
                        break;
                    },
                    _ => {
                        continue;
                    },
                }
            }
        });

        self.listener = Some(listener);
    }

    pub async fn fetch_leaf(
        &self,
        height: u64,
        source: TYPES::SignatureKey,
    ) -> anyhow::Result<Leaf2<TYPES>> {
        let leaf_request = Message {
            sender: self.public_key.clone(),
            kind: MessageKind::<TYPES>::External(
                bincode::serialize(&(height, self.public_key.clone()))
                    .expect("Failed to serialize leaf request"),
            ),
        };

        let mut network_receiver = self
            .network_receiver
            .clone()
            .expect("Tried to fetch leaf before calling `set_external_channel`");

        let serialized_leaf_request =
            BincodeSerializer::<StaticVersion<0, 0>>::serialize(&leaf_request)
                .expect("Failed to serialize leaf request");

        if let Err(e) =
            (self.network_functions.direct_message)(serialized_leaf_request, source).await
        {
            tracing::error!("Failed to send leaf request in test membership fetcher: {e}");
        };

        tokio::time::timeout(std::time::Duration::from_millis(100), async {
            loop {
                match network_receiver.recv_direct().await {
                    Ok(Event {
                        view_number: _,
                        event: EventType::ExternalMessageReceived { sender: _, data },
                    }) => {
                        let leaf: Leaf2<TYPES> = match bincode::deserialize(&data) {
                            Ok(message) => message,
                            Err(e) => {
                                tracing::debug!("Failed to deserialize message: {e:?}");
                                continue;
                            },
                        };

                        if leaf.height() == height {
                            return Ok(leaf);
                        }
                    },
                    Err(RecvError::Closed) => {
                        break Err(anyhow::anyhow!(
                            "Failed to fetch leaf: network task receiver closed"
                        ));
                    },
                    _ => {
                        continue;
                    },
                }
            }
        })
        .await
        .context("Leaf fetch timed out")?
    }
}
