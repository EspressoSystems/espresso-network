use std::sync::Arc;

use anyhow::{Error, bail};
use async_lock::RwLock;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use hotshot::{
    traits::{BlockPayload, TestableNodeImplementation},
    types::{Event, EventType},
};
use hotshot_example_types::block_types::TestTransaction;
use hotshot_testing::{
    test_runner::Node,
    test_task::{AnyTestTaskState, TestResult, TestTaskState, TestTaskStateSeed},
};
use hotshot_types::traits::{
    block_contents::BlockHeader,
    node_implementation::{NodeType, Versions},
};

use super::TransactionPayload;

#[derive(Clone, Debug)]
pub struct IncludedTransaction {
    pub payload: TransactionPayload,
    pub included: DateTime<Local>,
}

#[derive(Debug)]
pub struct BuilderValidationConfig {
    /// Number of transactions we expect to be included
    pub expected_txn_num: usize,
}

#[async_trait]
impl<Types, I, V> TestTaskStateSeed<Types, I, V> for BuilderValidationConfig
where
    Types: NodeType<Transaction = TestTransaction>,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    async fn into_state(
        self: Box<Self>,
        handles: Arc<RwLock<Vec<Node<Types, I, V>>>>,
    ) -> AnyTestTaskState<Types> {
        Box::new(BuilderValidationTask {
            config: *self,
            txn_history: Vec::new(),
            alien_transactions: Vec::new(),
            _handles: handles,
        })
    }
}

pub struct BuilderValidationTask<Types, I, V>
where
    Types: NodeType,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    pub(crate) _handles: Arc<RwLock<Vec<Node<Types, I, V>>>>,
    pub(crate) config: BuilderValidationConfig,
    pub(crate) txn_history: Vec<IncludedTransaction>,
    pub(crate) alien_transactions: Vec<TestTransaction>,
}

#[async_trait]
impl<Types, I, V> TestTaskState for BuilderValidationTask<Types, I, V>
where
    Types: NodeType<Transaction = TestTransaction>,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    type Event = Event<Types>;
    type Error = Error;

    async fn handle_event(&mut self, (event, node_id): (Self::Event, usize)) -> anyhow::Result<()> {
        // We only need to handle events from one node
        if node_id != 0 {
            return Ok(());
        }

        if let EventType::Decide { leaf_chain, .. } = event.event {
            for leaf in leaf_chain.iter().rev() {
                let payload = leaf
                    .leaf
                    .block_payload()
                    .expect("Node 0 didn't include payload");
                for transaction in payload.transactions(leaf.leaf.block_header().metadata()) {
                    if let Ok(txn_payload) = bincode::deserialize(transaction.bytes()) {
                        self.txn_history.push(IncludedTransaction {
                            payload: txn_payload,
                            included: Local::now(),
                        });
                    } else {
                        self.alien_transactions.push(transaction)
                    }
                }
            }
        }
        Ok(())
    }

    async fn check(&self) -> TestResult {
        if self.txn_history.len() + self.alien_transactions.len() <= self.config.expected_txn_num {
            return TestResult::Fail(Box::new(format!(
                "Expected at least {} transactions, got {}",
                self.config.expected_txn_num,
                self.txn_history.len()
            )));
        }

        if let Err(e) = self
            .txn_history
            .iter()
            .map(|txn| txn.payload.index as i64)
            .try_fold(-1, |prev, next| {
                if prev + 1 == next {
                    Ok(next)
                } else {
                    bail!("Transactions misordered: {} follows after {}", prev, next)
                }
            })
        {
            return TestResult::Fail(Box::new(e));
        };

        TestResult::Pass
    }
}
