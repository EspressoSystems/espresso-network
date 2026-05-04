// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use async_broadcast::{Receiver, RecvError};
use hotshot_types::{
    data::Leaf2,
    event::{Event, EventType},
    message::{Message, MessageKind},
    traits::{
        block_contents::BlockHeader, leaf_fetcher_network::LeafFetcherNetwork,
        node_implementation::NodeType,
    },
    vote::HasViewNumber,
};
use tokio::task::JoinHandle;
use vbs::{BinarySerializer, bincode_serializer::BincodeSerializer, version::StaticVersion};

use crate::storage_types::TestStorage;

pub struct Leaf2Fetcher<TYPES: NodeType> {
    pub network: Arc<dyn LeafFetcherNetwork<TYPES>>,
    pub storage: TestStorage<TYPES>,
    pub listener: Option<JoinHandle<()>>,
    pub public_key: TYPES::SignatureKey,
    pub network_receiver: Option<Receiver<Event<TYPES>>>,
}

impl<TYPES: NodeType> Leaf2Fetcher<TYPES> {
    pub fn new(
        network: Arc<dyn LeafFetcherNetwork<TYPES>>,
        storage: TestStorage<TYPES>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        Self {
            network,
            storage,
            listener: None,
            public_key,
            network_receiver: None,
        }
    }

    pub fn set_external_channel(&mut self, mut network_receiver: Receiver<Event<TYPES>>) {
        let public_key = self.public_key.clone();
        let storage = self.storage.clone();
        let network = self.network.clone();

        self.network_receiver = Some(network_receiver.clone());

        let listener = tokio::spawn(async move {
            loop {
                match network_receiver.recv_direct().await {
                    Ok(Event {
                        view_number: view,
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

                        if let Err(e) = network
                            .send_leaf_response(
                                view.u64().into(),
                                serialized_leaf_response,
                                requester,
                            )
                            .await
                        {
                            tracing::error!(
                                "Failed to send leaf response in test membership fetcher: {e}, \
                                 requested height: {requested_height}"
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
        let view = leaf_request.view_number();

        let leaves: BTreeMap<u64, Leaf2<TYPES>> = self
            .storage
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

        if let Some(leaf) = leaves.get(&height) {
            return Ok(leaf.clone());
        };
        tracing::debug!(
            "Leaf at height {height} not found in storage. Stored leaf heights: {heights:?}"
        );

        let mut network_receiver = self
            .network_receiver
            .clone()
            .expect("Tried to fetch leaf before calling `set_external_channel`");

        let serialized_leaf_request =
            BincodeSerializer::<StaticVersion<0, 0>>::serialize(&leaf_request)
                .expect("Failed to serialize leaf request");

        if let Err(e) = self
            .network
            .send_leaf_request(view.u64().into(), serialized_leaf_request, source)
            .await
        {
            tracing::error!("Failed to send leaf request in test membership fetcher: {e}");
        };

        tokio::time::timeout(std::time::Duration::from_millis(20), async {
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
