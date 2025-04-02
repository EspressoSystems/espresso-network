// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::unwrap_or_default)]
use std::{collections::BTreeMap, marker::PhantomData};

use async_broadcast::Sender;
use async_trait::async_trait;
use committable::Committable;
use hotshot_example_types::block_types::TestBlockHeader;
use hotshot_types::{
    data::Leaf2,
    event::{Event, EventType},
    message::UpgradeLock,
    traits::node_implementation::{ConsensusTime, NodeType, Versions},
};
use hotshot_utils::anytrace::*;
use tokio::task::JoinHandle;

use crate::{
    overall_safety_task::OverallSafetyPropertiesDescription,
    test_builder::TransactionValidator,
    test_task::{spawn_timeout_task, TestEvent, TestResult, TestTaskState},
};

/// Map from views to leaves for a single node, allowing multiple leaves for each view (because the node may a priori send us multiple leaves for a given view).
pub type NodeMap<TYPES> = BTreeMap<<TYPES as NodeType>::View, Vec<Leaf2<TYPES>>>;

/// A sanitized map from views to leaves for a single node, with only a single leaf per view.
pub type NodeMapSanitized<TYPES> = BTreeMap<<TYPES as NodeType>::View, Leaf2<TYPES>>;

/// Validate that the `NodeMap` only has a single leaf per view.
fn sanitize_node_map<TYPES: NodeType>(
    node_map: &NodeMap<TYPES>,
) -> Result<NodeMapSanitized<TYPES>> {
    let mut result = BTreeMap::new();

    for (view, leaves) in node_map.iter() {
        let mut reduced = leaves.clone();

        reduced.dedup();

        match reduced.len() {
            0 => {},
            1 => {
                result.insert(*view, reduced[0].clone());
            },
            _ => {
                bail!(
                    "We have received inconsistent leaves for view {view:?}. Leaves:\n\n{leaves:?}"
                );
            },
        }
    }

    Ok(result)
}

/// For a NodeMapSanitized, we validate that each leaf extends the preceding leaf.
async fn validate_node_map<TYPES: NodeType, V: Versions>(
    node_map: &NodeMapSanitized<TYPES>,
) -> Result<()> {
    // We first scan 3-chains to find an upgrade certificate that has reached a decide.
    let leaf_triples = node_map
        .values()
        .zip(node_map.values().skip(1))
        .zip(node_map.values().skip(2))
        .map(|((a, b), c)| (a, b, c));

    let mut decided_upgrade_certificate = None;
    let mut view_decided = TYPES::View::new(0);

    for (grandparent, _parent, child) in leaf_triples {
        if let Some(cert) = grandparent.upgrade_certificate() {
            if cert.data.decide_by <= child.view_number() {
                decided_upgrade_certificate = Some(cert);
                view_decided = child.view_number();

                break;
            }
        }
    }

    // To mimic consensus to use e.g. the `extends_upgrade` method,
    // we cannot immediately put the upgrade certificate in the lock.
    //
    // Instead, we initialize an empty lock and add the certificate in the appropriate view.
    let upgrade_lock = UpgradeLock::<TYPES, V>::new();

    let leaf_pairs = node_map.values().zip(node_map.values().skip(1));

    // Check that the child leaf follows the parent, possibly with a gap.
    for (parent, child) in leaf_pairs {
        ensure!(
              child.justify_qc().view_number >= parent.view_number(),
              "The node has provided leaf:\n\n{:?}\n\nbut its quorum certificate points to a view before the most recent leaf:\n\n{:?}",
              child,
              parent
        );

        child
            .extends_upgrade(parent, &upgrade_lock.decided_upgrade_certificate)
            .await
            .context(|e| {
                error!(
                    "Leaf {child:?} does not extend its parent {parent:?}: {}",
                    e
                )
            })?;

        ensure!(
          child.height() > parent.height(),
          "The node has decided leaf\n\n{:?}\n\nextending leaf\n\n{:?}but the block height did not increase.",
          child,
          parent
        );

        // We want to make sure the commitment matches,
        // but allow for the possibility that we may have skipped views in between.
        if child.justify_qc().view_number == parent.view_number()
            && child.justify_qc().data.leaf_commit != parent.commit()
        {
            bail!("The node has provided leaf:\n\n{child:?}\n\nwhich points to:\n\n{parent:?}\n\nbut the commits do not match.");
        }

        if child.view_number() == view_decided {
            upgrade_lock
                .decided_upgrade_certificate
                .write()
                .await
                .clone_from(&decided_upgrade_certificate);
        }
    }

    Ok(())
}

/// A map from node ids to `NodeMap`s; note that the latter may have multiple leaves per view in principle.
pub type NetworkMap<TYPES> = BTreeMap<usize, NodeMap<TYPES>>;

/// A map from node ids to `NodeMapSanitized`s; the latter has been sanitized validated to have a single leaf per view.
pub type NetworkMapSanitized<TYPES> = BTreeMap<usize, NodeMapSanitized<TYPES>>;

/// Validate that each node has only produced one unique leaf per view, and produce a `NetworkMapSanitized`.
fn sanitize_network_map<TYPES: NodeType>(
    network_map: &NetworkMap<TYPES>,
) -> Result<NetworkMapSanitized<TYPES>> {
    let mut result = BTreeMap::new();

    for (node, node_map) in network_map {
        result.insert(
            *node,
            sanitize_node_map(node_map)
                .context(|e| error!("Node {node} produced inconsistent leaves: {}", e))?,
        );
    }

    Ok(result)
}

pub type ViewMap<TYPES> = BTreeMap<<TYPES as NodeType>::View, BTreeMap<usize, Leaf2<TYPES>>>;

// Invert the network map by interchanging the roles of the node_id and view number.
//
// # Errors
//
// Returns an error if any node map is invalid.
async fn invert_network_map<TYPES: NodeType, V: Versions>(
    network_map: &NetworkMapSanitized<TYPES>,
) -> Result<ViewMap<TYPES>> {
    let mut inverted_map = BTreeMap::new();
    for (node_id, node_map) in network_map.iter() {
        validate_node_map::<TYPES, V>(node_map)
            .await
            .context(|e| error!("Node {node_id} has an invalid leaf history: {}", e))?;

        // validate each node's leaf map
        for (view, leaf) in node_map.iter() {
            let view_map = inverted_map.entry(*view).or_insert(BTreeMap::new());
            view_map.insert(*node_id, leaf.clone());
        }
    }

    Ok(inverted_map)
}

/// A view map, sanitized to have exactly one leaf per view.
pub type ViewMapSanitized<TYPES> = BTreeMap<<TYPES as NodeType>::View, Leaf2<TYPES>>;

fn sanitize_view_map<TYPES: NodeType>(
    view_map: &ViewMap<TYPES>,
) -> Result<ViewMapSanitized<TYPES>> {
    let mut result = BTreeMap::new();

    for (view, leaf_map) in view_map.iter() {
        let mut node_leaves: Vec<_> = leaf_map.iter().collect();

        node_leaves.dedup_by(|(_node_a, leaf_a), (_node_b, leaf_b)| leaf_a == leaf_b);

        ensure!(
            node_leaves.len() <= 1,
            error!(
                "The network does not agree on the following views: {}",
                leaf_map
                    .iter()
                    .fold(format!("\n\nView {view:?}:"), |acc, (node, leaf)| {
                        format!("{acc}\n\nNode {node} sent us leaf:\n\n{leaf:?}")
                    })
            )
        );

        if let Some(leaf) = node_leaves.first() {
            result.insert(*view, leaf.1.clone());
        }
    }

    for (parent, child) in result.values().zip(result.values().skip(1)) {
        // We want to make sure the aggregated leafmap has not missed a decide event
        if child.justify_qc().data.leaf_commit != parent.commit() {
            bail!("The network has decided:\n\n{child:?}\n\nwhich succeeds:\n\n{parent:?}\n\nbut the commits do not match. Did we miss an intermediate leaf?");
        }
    }

    Ok(result)
}

enum TestProgress {
    Incomplete,
    Finished,
}

/// Data availability task state
pub struct ConsistencyTask<TYPES: NodeType, V: Versions> {
    /// A map from node ids to (leaves keyed on view number)
    pub consensus_leaves: NetworkMap<TYPES>,
    /// safety task requirements
    pub safety_properties: OverallSafetyPropertiesDescription,
    /// whether we should have seen an upgrade certificate or not
    pub ensure_upgrade: bool,
    /// a list of errors accumulated by the task
    pub errors: Vec<Error>,
    /// channel used to shutdown the test
    pub test_sender: Sender<TestEvent>,
    /// phantom marker
    pub _pd: PhantomData<V>,
    /// function used to validate the number of transactions committed in each block
    pub validate_transactions: TransactionValidator,
    /// running timeout task
    pub timeout_task: JoinHandle<()>,
}

impl<TYPES: NodeType<BlockHeader = TestBlockHeader>, V: Versions> ConsistencyTask<TYPES, V> {
    pub async fn validate(&self) -> Result<()> {
        let sanitized_network_map = sanitize_network_map(&self.consensus_leaves)?;

        let inverted_map = invert_network_map::<TYPES, V>(&sanitized_network_map).await?;

        let sanitized_view_map = sanitize_view_map(&inverted_map)?;
        let num_successful_views = sanitized_view_map.iter().len();

        // check that we've succeeded in enough views
        ensure!(
            num_successful_views >= self.safety_properties.num_successful_views,
            "Not enough successful views: expected {:?} but got {:?}",
            self.safety_properties.num_successful_views,
            num_successful_views,
        );

        let expected_upgrade = self.ensure_upgrade;
        let actual_upgrade = sanitized_view_map.iter().fold(false, |acc, (_view, leaf)| {
            acc || leaf.upgrade_certificate().is_some()
        });

        let mut transactions = Vec::new();

        transactions = sanitized_view_map
            .iter()
            .fold(transactions, |mut acc, (view, leaf)| {
                acc.push((**view, leaf.block_header().metadata.num_transactions));

                acc
            });

        (self.validate_transactions)(&transactions)?;

        ensure!(
          expected_upgrade == actual_upgrade,
          "Mismatch between expected and actual upgrade. Expected upgrade: {expected_upgrade}. Actual upgrade: {actual_upgrade}"
        );

        Ok(())
    }

    async fn partial_validate(&self) -> Result<TestProgress> {
        self.check_view_success().await?;
        self.check_view_failure().await?;

        self.check_total_successes().await
    }

    fn add_error(&mut self, error: Error) {
        self.errors.push(error);
    }

    async fn handle_result(&mut self, result: Result<TestProgress>) {
        match result {
            Ok(TestProgress::Finished) => {
                let _ = self.test_sender.broadcast(TestEvent::Shutdown).await;
            },
            Err(e) => {
                self.add_error(e);
                let _ = self.test_sender.broadcast(TestEvent::Shutdown).await;
            },
            Ok(TestProgress::Incomplete) => {},
        }
    }

    async fn check_total_successes(&self) -> Result<TestProgress> {
        let sanitized_network_map = sanitize_network_map(&self.consensus_leaves)?;

        let inverted_map = invert_network_map::<TYPES, V>(&sanitized_network_map).await?;

        if inverted_map.len() >= self.safety_properties.num_successful_views {
            Ok(TestProgress::Finished)
        } else {
            Ok(TestProgress::Incomplete)
        }
    }
    pub async fn check_view_success(&self) -> Result<()> {
        for (node_id, node_map) in self.consensus_leaves.iter() {
            for (view, leaf) in node_map {
                ensure!(
                    !self
                        .safety_properties
                        .expected_view_failures
                        .contains(view),
                    "Expected a view failure, but got a decided leaf for view {:?} from node {:?}.\n\nLeaf:\n\n{:?}",
                    view,
                    node_id,
                    leaf
                );
            }
        }

        Ok(())
    }

    pub async fn check_view_failure(&self) -> Result<()> {
        let sanitized_network_map = sanitize_network_map(&self.consensus_leaves)?;

        let mut inverted_map = invert_network_map::<TYPES, V>(&sanitized_network_map).await?;

        let (current_view, _) = inverted_map
            .pop_last()
            .context(error!("Leaf map is empty, which should be impossible"))?;
        let Some((last_view, _)) = inverted_map.pop_last() else {
            // the view cannot fail if there wasn't a prior view in the map.
            return Ok(());
        };

        // filter out views we expected to (possibly) fail
        let unexpected_failed_views: Vec<_> = (*(last_view + 1)..*current_view)
            .filter(|view| {
                !self.safety_properties.expected_view_failures.contains(view)
                    && !self.safety_properties.possible_view_failures.contains(view)
            })
            .collect();

        ensure!(
            unexpected_failed_views.is_empty(),
            "Unexpected failed views: {:?}",
            unexpected_failed_views
        );

        Ok(())
    }
}

#[async_trait]
impl<TYPES: NodeType<BlockHeader = TestBlockHeader>, V: Versions> TestTaskState
    for ConsistencyTask<TYPES, V>
{
    type Event = Event<TYPES>;
    type Error = Error;

    /// Handles an event from one of multiple receivers.
    async fn handle_event(&mut self, (message, id): (Self::Event, usize)) -> Result<()> {
        if let Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        } = message
        {
            {
                let mut timeout_task = spawn_timeout_task(
                    self.test_sender.clone(),
                    self.safety_properties.decide_timeout,
                );

                std::mem::swap(&mut self.timeout_task, &mut timeout_task);

                timeout_task.abort();
            }

            for leaf_info in leaf_chain.iter().rev() {
                let map = &mut self.consensus_leaves.entry(id).or_insert(BTreeMap::new());

                map.entry(leaf_info.leaf.view_number())
                    .and_modify(|vec| vec.push(leaf_info.leaf.clone()))
                    .or_insert(vec![leaf_info.leaf.clone()]);

                let result = self.partial_validate().await;

                self.handle_result(result).await;
            }
        }

        Ok(())
    }

    async fn check(&self) -> TestResult {
        self.timeout_task.abort();

        let mut errors: Vec<_> = self.errors.iter().map(|e| e.to_string()).collect();

        if let Err(e) = self.validate().await {
            errors.push(e.to_string());
        }

        if !errors.is_empty() {
            TestResult::Fail(Box::new(errors))
        } else {
            TestResult::Pass
        }
    }
}
