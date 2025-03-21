// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use async_broadcast::Receiver;
use async_lock::RwLock;
use hotshot::traits::TestableNodeImplementation;
use hotshot_types::traits::node_implementation::{NodeType, Versions};
use rand::thread_rng;
use tokio::{spawn, task::JoinHandle, time::sleep};

use crate::{test_runner::Node, test_task::TestEvent};

// the obvious idea here is to pass in a "stream" that completes every `n` seconds
// the stream construction can definitely be fancier but that's the baseline idea

/// state of task that decides when things are completed
pub struct TxnTask<TYPES: NodeType, I: TestableNodeImplementation<TYPES>, V: Versions> {
    // TODO should this be in a rwlock? Or maybe a similar abstraction to the registry is in order
    /// Handles for all nodes.
    pub handles: Arc<RwLock<Vec<Node<TYPES, I, V>>>>,
    /// Optional index of the next node.
    pub next_node_idx: Option<usize>,
    /// time to wait between txns
    pub duration: Duration,
    /// Receiver for the shutdown signal from the testing harness
    pub shutdown_chan: Receiver<TestEvent>,
}

impl<TYPES: NodeType, I: TestableNodeImplementation<TYPES>, V: Versions> TxnTask<TYPES, I, V> {
    pub fn run(mut self) -> JoinHandle<()> {
        spawn(async move {
            loop {
                sleep(self.duration).await;
                if let Ok(TestEvent::Shutdown) = self.shutdown_chan.try_recv() {
                    break;
                }
                self.submit_tx().await;
            }
        })
    }
    async fn submit_tx(&mut self) {
        if let Some(idx) = self.next_node_idx {
            let handles = &self.handles.read().await;
            // submit to idx handle
            // increment state
            self.next_node_idx = Some((idx + 1) % handles.len());
            match handles.get(idx) {
                None => {
                    tracing::error!("couldn't get node in txn task");
                },
                Some(node) => {
                    // use rand::seq::IteratorRandom;
                    // we're assuming all nodes have the same leaf.
                    // If they don't match, this is probably fine since
                    // it should be caught by an assertion (and the txn will be rejected anyway)
                    let leaf = node.handle.decided_leaf().await;
                    let txn = I::leaf_create_random_transaction(&leaf, &mut thread_rng(), 0);
                    node.handle
                        .submit_transaction(txn.clone())
                        .await
                        .expect("Could not send transaction");
                },
            }
        }
    }
}

/// build the transaction task
#[derive(Clone, Debug)]
pub enum TxnTaskDescription {
    /// submit transactions in a round robin style using
    /// every `Duration` seconds
    RoundRobinTimeBased(Duration),
    /// TODO
    DistributionBased, // others?
}
