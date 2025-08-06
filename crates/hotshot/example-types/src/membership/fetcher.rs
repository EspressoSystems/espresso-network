// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use anyhow::Context;
use hotshot::traits::{NodeImplementation, TestableNodeImplementation};
use hotshot_types::{
    data::Leaf2,
    traits::{
        block_contents::BlockHeader,
        network::{AsyncGenerator, ConnectedNetwork},
        node_implementation::{NodeType, Versions},
    },
    HotShotConfig, ValidatorConfig,
};
use tokio::task::JoinHandle;

use crate::storage_types::TestStorage;

pub struct Leaf2Fetcher<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> {
    network: Arc<<I as NodeImplementation<TYPES>>::Network>,
    storage: TestStorage<TYPES>,
    listener: JoinHandle<()>,
    public_key: TYPES::SignatureKey,
}

impl<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> Leaf2Fetcher<TYPES, I> {
    pub fn new(
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        storage: TestStorage<TYPES>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        let storage_clone = storage.clone();
        let network_clone = network.clone();
        let listener = tokio::spawn(async move {
            while let Ok(message) = network_clone.recv_message().await {
                // Deserialize the message
                let (requested_height, requester): (u64, TYPES::SignatureKey) =
                    match bincode::deserialize(&message) {
                        Ok(message) => message,
                        Err(e) => {
                            tracing::error!("Failed to deserialize message: {:?}", e);
                            continue;
                        },
                    };

                let leaves: BTreeMap<u64, Leaf2<TYPES>> = storage_clone
                    .inner
                    .read()
                    .await
                    .proposals2
                    .iter()
                    .map(|(view, proposal)| {
                        (
                            proposal.data.block_header.block_number(),
                            Leaf2::from_quorum_proposal(&proposal.data.clone().into()),
                        )
                    })
                    .collect();

                let Some(leaf) = leaves.get(&requested_height) else {
                    tracing::warn!("Block at height {} not found in storage", requested_height);
                    continue;
                };

                let serialized_leaf = bincode::serialize(&leaf).expect("Failed to serialized leaf");

                network_clone
                    .direct_message(serialized_leaf, requester)
                    .await;
            }
        });

        Self {
            network,
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
        self.network
            .direct_message(serialized_leaf_request, source)
            .await;

        tokio::time::timeout(std::time::Duration::from_secs(6), async {
            while let Ok(message) = self.network.recv_message().await {
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
