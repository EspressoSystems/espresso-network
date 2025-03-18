// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use async_broadcast::{broadcast, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use futures::{future::BoxFuture, Stream, StreamExt};
use hotshot::{
    traits::BlockPayload,
    types::{Event, EventType, SignatureKey},
};
use hotshot_builder_api::{
    v0_1::{
        self,
        block_info::{AvailableBlockData, AvailableBlockInfo},
        builder::{BuildError, Error, Options},
    },
    v0_2::block_info::AvailableBlockHeaderInputV1,
    v0_99,
};
use hotshot_types::{
    bundle::Bundle,
    constants::{LEGACY_BUILDER_MODULE, MARKETPLACE_BUILDER_MODULE},
    data::VidCommitment,
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};
use lru::LruCache;
use tide_disco::{method::ReadState, App, Url};
use tokio::spawn;
use vbs::version::StaticVersionType;

use super::{build_block, run_builder_source, BlockEntry, BuilderTask, TestBuilderImplementation};
use crate::test_builder::BuilderChange;

pub struct SimpleBuilderImplementation;

impl SimpleBuilderImplementation {
    pub async fn create<TYPES: NodeType>(
        num_nodes: usize,
        changes: HashMap<u64, BuilderChange>,
        change_sender: Sender<BuilderChange>,
    ) -> (SimpleBuilderSource<TYPES>, SimpleBuilderTask<TYPES>) {
        let (pub_key, priv_key) =
            TYPES::BuilderSignatureKey::generated_from_seed_indexed([1; 32], 0);

        let transactions = Arc::new(RwLock::new(HashMap::new()));
        let blocks = Arc::new(RwLock::new(HashMap::new()));
        let should_fail_claims = Arc::new(AtomicBool::new(false));

        let source = SimpleBuilderSource {
            pub_key,
            priv_key,
            transactions: transactions.clone(),
            blocks: blocks.clone(),
            num_nodes: Arc::new(RwLock::new(num_nodes)),
            should_fail_claims: Arc::clone(&should_fail_claims),
        };

        let task = SimpleBuilderTask {
            transactions,
            blocks,
            decided_transactions: LruCache::new(NonZeroUsize::new(u16::MAX.into()).expect("> 0")),
            should_fail_claims,
            change_sender,
            changes,
        };

        (source, task)
    }
}

#[async_trait]
impl<TYPES: NodeType> TestBuilderImplementation<TYPES> for SimpleBuilderImplementation
where
    <TYPES as NodeType>::InstanceState: Default,
{
    type Config = ();

    async fn start(
        num_nodes: usize,
        url: Url,
        _config: Self::Config,
        changes: HashMap<u64, BuilderChange>,
    ) -> Box<dyn BuilderTask<TYPES>> {
        let (change_sender, change_receiver) = broadcast(128);
        let (source, task) = Self::create(num_nodes, changes, change_sender).await;
        run_builder_source(url, change_receiver, source);

        Box::new(task)
    }
}

#[derive(Debug, Clone)]
pub struct SimpleBuilderSource<TYPES: NodeType> {
    pub_key: TYPES::BuilderSignatureKey,
    priv_key: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,
    num_nodes: Arc<RwLock<usize>>,
    #[allow(clippy::type_complexity)]
    transactions: Arc<RwLock<HashMap<Commitment<TYPES::Transaction>, SubmittedTransaction<TYPES>>>>,
    blocks: Arc<RwLock<HashMap<BuilderCommitment, BlockEntry<TYPES>>>>,
    should_fail_claims: Arc<AtomicBool>,
}

#[async_trait]
impl<TYPES: NodeType> ReadState for SimpleBuilderSource<TYPES> {
    type State = Self;

    async fn read<T>(
        &self,
        op: impl Send + for<'a> FnOnce(&'a Self::State) -> BoxFuture<'a, T> + 'async_trait,
    ) -> T {
        op(self).await
    }
}

#[async_trait]
impl<TYPES: NodeType> v0_99::data_source::BuilderDataSource<TYPES> for SimpleBuilderSource<TYPES>
where
    <TYPES as NodeType>::InstanceState: Default,
{
    /// To get the list of available blocks
    async fn bundle(
        &self,
        _parent_view: u64,
        _parent_hash: &VidCommitment,
        view_number: u64,
    ) -> Result<Bundle<TYPES>, BuildError> {
        let transactions = self
            .transactions
            .read(|txns| {
                Box::pin(async {
                    txns.values()
                        .filter(|txn| {
                            // We want transactions that are either unclaimed, or claimed long ago
                            // and thus probably not included, or they would've been decided on
                            // already and removed from the queue
                            txn.claimed
                                .map(|claim_time| claim_time.elapsed() > Duration::from_secs(30))
                                .unwrap_or(true)
                        })
                        .cloned()
                        .map(|txn| txn.transaction)
                        .collect::<Vec<TYPES::Transaction>>()
                })
            })
            .await;

        let fee_amount = 1;
        let sequencing_fee: BuilderFee<TYPES> = BuilderFee {
            fee_amount,
            fee_account: self.pub_key.clone(),
            fee_signature: TYPES::BuilderSignatureKey::sign_sequencing_fee_marketplace(
                &self.priv_key.clone(),
                fee_amount,
                view_number,
            )
            .expect("Failed to sign fee!"),
        };

        let signature =
            TYPES::BuilderSignatureKey::sign_bundle::<TYPES>(&self.priv_key, &transactions)
                .unwrap();

        {
            // claim transactions
            let mut transactions_lock = self.transactions.write().await;
            let transaction_hashes = transactions.iter().map(|txn| txn.commit());
            let time = Instant::now();

            for hash in transaction_hashes {
                if let Some(txn) = transactions_lock.get_mut(&hash) {
                    txn.claimed = Some(time);
                }
            }
        }

        Ok(Bundle {
            transactions,
            signature,
            sequencing_fee,
        })
    }

    /// To get the builder's address
    async fn builder_address(&self) -> Result<TYPES::BuilderSignatureKey, BuildError> {
        Ok(self.pub_key.clone())
    }
}

#[async_trait]
impl<TYPES: NodeType> v0_1::data_source::BuilderDataSource<TYPES> for SimpleBuilderSource<TYPES>
where
    <TYPES as NodeType>::InstanceState: Default,
{
    async fn available_blocks(
        &self,
        _for_parent: &VidCommitment,
        _view_number: u64,
        _sender: TYPES::SignatureKey,
        _signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<TYPES>>, BuildError> {
        let transactions = self
            .transactions
            .read(|txns| {
                Box::pin(async {
                    txns.values()
                        .filter(|txn| {
                            // We want transactions that are either unclaimed, or claimed long ago
                            // and thus probably not included, or they would've been decided on
                            // already and removed from the queue
                            txn.claimed
                                .map(|claim_time| claim_time.elapsed() > Duration::from_secs(30))
                                .unwrap_or(true)
                        })
                        .cloned()
                        .map(|txn| txn.transaction)
                        .collect::<Vec<TYPES::Transaction>>()
                })
            })
            .await;

        if transactions.is_empty() {
            // We don't want to return an empty block if we have no transactions, as we would end up
            // driving consensus to produce empty blocks extremely quickly when mempool is empty.
            // Instead, we return no blocks, so that view leader will keep asking for blocks until
            // either we have something non-trivial to propose, or leader runs out of time to propose,
            // in which case view leader will finally propose an empty block themselves.
            return Ok(vec![]);
        }

        let block_entry =
            build_block::<TYPES>(transactions, self.pub_key.clone(), self.priv_key.clone()).await;

        let metadata = block_entry.metadata.clone();

        self.blocks
            .write()
            .await
            .insert(block_entry.metadata.block_hash.clone(), block_entry);

        Ok(vec![metadata])
    }

    async fn claim_block(
        &self,
        block_hash: &BuilderCommitment,
        _view_number: u64,
        _sender: TYPES::SignatureKey,
        _signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<TYPES>, BuildError> {
        if self.should_fail_claims.load(Ordering::Relaxed) {
            return Err(BuildError::Missing);
        }

        let payload = {
            let mut blocks = self.blocks.write().await;
            let entry = blocks.get_mut(block_hash).ok_or(BuildError::NotFound)?;
            entry.payload.take().ok_or(BuildError::Missing)?
        };

        let now = Instant::now();

        let claimed_transactions = payload
            .block_payload
            .transaction_commitments(&payload.metadata);

        let mut transactions = self.transactions.write().await;
        for txn_hash in claimed_transactions {
            if let Some(txn) = transactions.get_mut(&txn_hash) {
                txn.claimed = Some(now);
            }
        }

        Ok(payload)
    }

    async fn claim_block_with_num_nodes(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        num_nodes: usize,
    ) -> Result<AvailableBlockData<TYPES>, BuildError> {
        *self.num_nodes.write().await = num_nodes;
        self.claim_block(block_hash, view_number, sender, signature)
            .await
    }

    async fn claim_block_header_input(
        &self,
        block_hash: &BuilderCommitment,
        _view_number: u64,
        _sender: TYPES::SignatureKey,
        _signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<TYPES>, BuildError> {
        if self.should_fail_claims.load(Ordering::Relaxed) {
            return Err(BuildError::Missing);
        }

        let mut blocks = self.blocks.write().await;
        let entry = blocks.get_mut(block_hash).ok_or(BuildError::NotFound)?;
        entry.header_input.take().ok_or(BuildError::Missing)
    }

    async fn builder_address(&self) -> Result<TYPES::BuilderSignatureKey, BuildError> {
        Ok(self.pub_key.clone())
    }
}

impl<TYPES: NodeType> SimpleBuilderSource<TYPES> {
    pub async fn run(self, url: Url)
    where
        <TYPES as NodeType>::InstanceState: Default,
    {
        let builder_api_0_1 = hotshot_builder_api::v0_1::builder::define_api::<
            SimpleBuilderSource<TYPES>,
            TYPES,
        >(&Options::default())
        .expect("Failed to construct the builder API");

        let builder_api_0_3 = hotshot_builder_api::v0_99::builder::define_api::<
            SimpleBuilderSource<TYPES>,
            TYPES,
        >(&Options::default())
        .expect("Failed to construct the builder API");

        let mut app: App<SimpleBuilderSource<TYPES>, Error> = App::with_state(self);
        app.register_module::<Error, _>(LEGACY_BUILDER_MODULE, builder_api_0_1)
            .expect("Failed to register builder API 0.1")
            .register_module::<Error, _>(MARKETPLACE_BUILDER_MODULE, builder_api_0_3)
            .expect("Failed to register builder API 0.3");

        spawn(app.serve(url, hotshot_builder_api::v0_1::Version::instance()));
    }
}

#[derive(Debug, Clone)]
struct SubmittedTransaction<TYPES: NodeType> {
    claimed: Option<Instant>,
    transaction: TYPES::Transaction,
}

#[derive(Clone)]
pub struct SimpleBuilderTask<TYPES: NodeType> {
    #[allow(clippy::type_complexity)]
    transactions: Arc<RwLock<HashMap<Commitment<TYPES::Transaction>, SubmittedTransaction<TYPES>>>>,
    blocks: Arc<RwLock<HashMap<BuilderCommitment, BlockEntry<TYPES>>>>,
    decided_transactions: LruCache<Commitment<TYPES::Transaction>, ()>,
    should_fail_claims: Arc<AtomicBool>,
    changes: HashMap<u64, BuilderChange>,
    change_sender: Sender<BuilderChange>,
}

impl<TYPES: NodeType> BuilderTask<TYPES> for SimpleBuilderTask<TYPES> {
    fn start(
        mut self: Box<Self>,
        mut stream: Box<dyn Stream<Item = Event<TYPES>> + std::marker::Unpin + Send + 'static>,
    ) {
        spawn(async move {
            let mut should_build_blocks = true;
            loop {
                match stream.next().await {
                    None => {
                        break;
                    },
                    Some(evt) => match evt.event {
                        EventType::ViewFinished { view_number } => {
                            if let Some(change) = self.changes.remove(&view_number) {
                                match change {
                                    BuilderChange::Up => should_build_blocks = true,
                                    BuilderChange::Down => {
                                        should_build_blocks = false;
                                        self.transactions.write().await.clear();
                                        self.blocks.write().await.clear();
                                    },
                                    BuilderChange::FailClaims(value) => {
                                        self.should_fail_claims.store(value, Ordering::Relaxed);
                                    },
                                }
                                let _ = self.change_sender.broadcast(change).await;
                            }
                        },
                        EventType::Decide { leaf_chain, .. } if should_build_blocks => {
                            let mut queue = self.transactions.write().await;
                            for leaf_info in leaf_chain.iter() {
                                if let Some(ref payload) = leaf_info.leaf.block_payload() {
                                    for txn in payload.transaction_commitments(
                                        leaf_info.leaf.block_header().metadata(),
                                    ) {
                                        self.decided_transactions.put(txn, ());
                                        queue.remove(&txn);
                                    }
                                }
                            }
                            self.blocks.write().await.clear();
                        },
                        EventType::DaProposal { proposal, .. } if should_build_blocks => {
                            let payload = TYPES::BlockPayload::from_bytes(
                                &proposal.data.encoded_transactions,
                                &proposal.data.metadata,
                            );
                            let now = Instant::now();

                            let mut queue = self.transactions.write().await;
                            for commitment in
                                payload.transaction_commitments(&proposal.data.metadata)
                            {
                                if let Some(txn) = queue.get_mut(&commitment) {
                                    txn.claimed = Some(now);
                                }
                            }
                        },
                        EventType::Transactions { transactions } if should_build_blocks => {
                            let mut queue = self.transactions.write().await;
                            for transaction in transactions {
                                if !self.decided_transactions.contains(&transaction.commit()) {
                                    queue.insert(
                                        transaction.commit(),
                                        SubmittedTransaction {
                                            claimed: None,
                                            transaction: transaction.clone(),
                                        },
                                    );
                                }
                            }
                        },
                        _ => {},
                    },
                }
            }
        });
    }
}
