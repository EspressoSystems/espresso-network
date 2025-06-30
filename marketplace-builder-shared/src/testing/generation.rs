use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::{Error, Result};
use async_lock::RwLock;
use async_trait::async_trait;
use chrono::Local;
use hotshot::{
    traits::TestableNodeImplementation,
    types::{Event, EventType},
};
use hotshot_builder_api::v0_2::builder;
use hotshot_example_types::block_types::TestTransaction;
use hotshot_testing::{
    test_runner::Node,
    test_task::{AnyTestTaskState, TestResult, TestTaskState, TestTaskStateSeed},
};
use hotshot_types::traits::node_implementation::{NodeType, Versions};
use rand::{Rng, seq::SliceRandom, thread_rng};
use surf_disco::Client;
use url::Url;
use vec1::Vec1;

use super::TransactionPayload;

const FLOOD_INCREMENT: usize = 1000;

/// Strategy for generating transactions
/// - `Random` submits a certain amount of random transactions every view
/// - `Seeded` accepts a map of transactions to submit on each view
/// - `Flood` continually submits transactions, re-trying if it encounters an error on
///   private mempool API.
/// <div class="warning">
/// Using [`Flood`] with [`SubmissionEndpoint::Public`] can overload network with transactions,
/// because there's no form of backpressure on gossiped transactions
/// </div>
#[derive(Debug)]
pub enum GenerationStrategy {
    Random {
        /// Minimum number of transactions to submit per view
        min_per_view: usize,
        /// Maximum number of transactions to submit per view
        max_per_view: usize,
        /// Minimum size of transaction to generate
        min_tx_size: usize,
        /// Maximum size of transaction to generate
        max_tx_size: usize,
    },
    Seeded(HashMap<u64, Vec<TransactionPayload>>),
    Flood {
        /// Minimum size of transaction to generate
        min_tx_size: usize,
        /// Maximum size of transaction to generate
        max_tx_size: usize,
    },
}

/// Strategy for transaction submission
/// `Public` will use public mempool, submitting to a node with given index
/// `Private` will use private mempool
#[derive(Debug)]
pub enum SubmissionEndpoint {
    Public {
        /// Hotshot node index to submit transactions to
        node_idx: usize,
    },
    Private,
}

/// Configuration for transaction generation task
#[derive(Debug)]
pub struct TransactionGenerationConfig {
    /// Strategy to use when generating transactions
    pub strategy: GenerationStrategy,
    /// Where to submit transactions
    pub endpoints: Vec<SubmissionEndpoint>,
}

#[async_trait]
impl<Types, I, V> TestTaskStateSeed<Types, I, V> for TransactionGenerationConfig
where
    Types: NodeType<Transaction = TestTransaction>,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    async fn into_state(
        self: Box<Self>,
        handles: Arc<RwLock<Vec<Node<Types, I, V>>>>,
    ) -> AnyTestTaskState<Types> {
        let builder_urls = handles.read_arc().await[0]
            .handle
            .hotshot
            .config
            .builder_urls
            .clone();
        Box::new(TransactionGenerationTask {
            builder_urls,
            config: *self,
            handles,
            txn_nonce: 0,
            txn_queue: VecDeque::new(),
        })
    }
}

pub struct TransactionGenerationTask<Types, I, V>
where
    Types: NodeType,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    pub(crate) builder_urls: Vec1<Url>,
    pub(crate) config: TransactionGenerationConfig,
    pub(crate) handles: Arc<RwLock<Vec<Node<Types, I, V>>>>,
    pub(crate) txn_nonce: usize,
    pub(crate) txn_queue: VecDeque<TestTransaction>,
}

impl<Types, I, V> TransactionGenerationTask<Types, I, V>
where
    Types: NodeType<Transaction = TestTransaction>,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    /// Fill queue of transactions to submit for `view_number` based on generation strategy
    fn fill_queue(&mut self, view_number: u64) {
        match &self.config.strategy {
            GenerationStrategy::Seeded(transactions) => {
                if let Some(transactions) = transactions.get(&view_number) {
                    self.txn_nonce += transactions.len();
                    self.txn_queue.extend(
                        transactions
                            .iter()
                            .map(TryFrom::try_from)
                            .map(Result::unwrap),
                    );
                }
            },
            GenerationStrategy::Random {
                min_per_view,
                max_per_view,
                min_tx_size,
                max_tx_size,
            } => {
                let mut rng = thread_rng();
                let n_txn = rng.gen_range(*min_per_view..=*max_per_view);
                self.txn_queue.reserve(n_txn + self.txn_queue.len());

                let now = Local::now();
                for _ in 0..n_txn {
                    let txn_size = rng.gen_range(*min_tx_size..=*max_tx_size);
                    let mut data = vec![0; txn_size];
                    rng.fill(data.as_mut_slice());
                    let transaction = TransactionPayload {
                        index: self.txn_nonce,
                        created: now,
                        data,
                    };

                    self.txn_queue
                        .push_back(TestTransaction::try_from(&transaction).unwrap());

                    self.txn_nonce += 1;
                }
            },
            GenerationStrategy::Flood {
                min_tx_size,
                max_tx_size,
            } => {
                let mut rng = thread_rng();
                let now = Local::now();

                for _ in 0..FLOOD_INCREMENT.saturating_sub(self.txn_queue.len()) {
                    let txn_size = rng.gen_range(*min_tx_size..=*max_tx_size);
                    let mut data = vec![0; txn_size];
                    rng.fill(data.as_mut_slice());

                    let transaction = TransactionPayload {
                        index: self.txn_nonce,
                        created: now,
                        data,
                    };

                    self.txn_queue
                        .push_back(TestTransaction::try_from(&transaction).unwrap());

                    self.txn_nonce += 1;
                }
            },
        };
    }
}

#[async_trait]
impl<Types, I, V> TestTaskState for TransactionGenerationTask<Types, I, V>
where
    Types: NodeType<Transaction = TestTransaction>,
    I: TestableNodeImplementation<Types>,
    V: Versions,
{
    type Event = Event<Types>;
    type Error = Error;

    async fn handle_event(&mut self, (event, node_id): (Self::Event, usize)) -> Result<()> {
        // We only need to handle events from one node
        if node_id != 0 {
            return Ok(());
        }

        if let EventType::ViewFinished { view_number } = event.event {
            self.fill_queue(*view_number);

            let url = self.builder_urls.first().clone();
            let private_mempool_client = Client::<
                hotshot_builder_api::v0_1::builder::Error,
                hotshot_builder_api::v0_1::Version,
            >::new(url.join("txn_submit").unwrap());

            while let Some(txn) = self.txn_queue.pop_front() {
                let endpoint = {
                    let mut rng = thread_rng();
                    self.config
                        .endpoints
                        .choose(&mut rng)
                        .unwrap_or(&SubmissionEndpoint::Private)
                };

                match endpoint {
                    SubmissionEndpoint::Public { node_idx } => {
                        self.handles.read_arc().await[*node_idx]
                            .handle
                            .hotshot
                            .publish_transaction_async(txn)
                            .await
                            .expect("Failed to submit transaction to public mempool");
                    },
                    SubmissionEndpoint::Private => {
                        if let Err(e) = private_mempool_client
                            .post::<()>("submit")
                            .body_binary(&txn)
                            .unwrap()
                            .send()
                            .await
                        {
                            match e {
                                // If we can't reach the builder altogether, test should fail
                                builder::Error::Request(request_error) => {
                                    panic!("Builder API not available: {request_error}")
                                },
                                // If the builder returns an error, we will re-submit this transaction
                                // on the next view, so we return it to the queue and break
                                error => {
                                    tracing::warn!(?error, "Builder API error");
                                    self.txn_queue.push_front(txn);
                                    break;
                                },
                            };
                        }
                    },
                }
            }
        }

        Ok(())
    }

    async fn check(&self) -> TestResult {
        TestResult::Pass
    }
}
