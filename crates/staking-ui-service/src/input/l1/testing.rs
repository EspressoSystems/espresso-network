#![cfg(test)]

use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use alloy::primitives::keccak256;
use async_lock::RwLockReadGuard;
use tagged_base64::TaggedBase64;
use tide_disco::{Error as _, StatusCode};
use tokio::{task::spawn, time::sleep};

use crate::types::common::{NodeSetEntry, Ratio};

use super::*;

/// Easy-setup storage that just uses memory.
#[derive(Clone, Debug, Default)]
pub(crate) struct MemoryStorage {
    snapshot: Arc<RwLock<Option<PersistentSnapshot>>>,
}

impl L1Persistence for MemoryStorage {
    async fn finalized_snapshot(&self) -> Result<Option<PersistentSnapshot>> {
        Ok(self.snapshot.read().await.clone())
    }

    async fn save_genesis(&self, snapshot: PersistentSnapshot) -> Result<()> {
        *self.snapshot.write().await = Some(snapshot);
        Ok(())
    }

    async fn apply_events(
        &self,
        block: L1BlockId,
        timestamp: Timestamp,
        node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> Result<()> {
        let mut lock = self.snapshot.write().await;
        let snapshot = lock.get_or_insert(PersistentSnapshot::genesis(block_id(0), 0));

        for diff in node_set_diff {
            apply_node_set_diff(&mut snapshot.node_set, &diff);
        }
        for (address, diff) in wallets_diff {
            apply_wallet_diff(&mut snapshot.wallets, address, &diff);
        }
        snapshot.block = block;
        snapshot.timestamp = timestamp;

        Ok(())
    }
}

/// Storage that always fails.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FailStorage;

impl L1Persistence for FailStorage {
    async fn finalized_snapshot(&self) -> Result<Option<PersistentSnapshot>> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }

    async fn save_genesis(&self, _snapshot: PersistentSnapshot) -> Result<()> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }

    async fn apply_events(
        &self,
        _block: L1BlockId,
        _timestamp: Timestamp,
        _node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        _wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> Result<()> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }
}

/// Resettable stream that yields a predefined list of inputs.
#[derive(Clone, Debug)]
pub(crate) struct VecStream {
    inputs: Vec<BlockInput>,
    reorg: Option<Vec<BlockInput>>,
    pos: usize,
    panic_at_end: bool,
}

impl Default for VecStream {
    fn default() -> Self {
        Self {
            inputs: vec![],
            reorg: None,
            pos: 0,
            panic_at_end: true,
        }
    }
}

impl VecStream {
    /// Emulate an infinite stream.
    ///
    /// The resulting stream will block indefinitely when it reaches the end of its predefined
    /// input sequence.
    pub(crate) fn infinite() -> Self {
        Self {
            panic_at_end: false,
            ..Default::default()
        }
    }

    /// Append a new L1 block input to be yielded by the stream.
    pub(crate) fn push(&mut self, input: BlockInput) {
        self.inputs.push(input);
    }

    /// Provide an alternative sequence of inputs to yield after the stream is reset.
    pub(crate) fn with_reorg(mut self, inputs: Vec<BlockInput>) -> Self {
        self.reorg = Some(inputs);
        self
    }
}

impl Stream for VecStream {
    type Item = BlockInput;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.pos >= self.inputs.len() {
            if self.panic_at_end {
                // Most tests expect to never hit the end of the predefined input sequence. If
                // we do, panic and fail the test.
                panic!("reached end of predefined input stream");
            } else {
                // In some cases, we want to emulate the realistic behavior of an L1 stream,
                // which is blocking when no more blocks are readily available.
                tracing::warn!("reached end of predefined input stream, blocking indefinitely");
                return Poll::Pending;
            }
        }

        let poll = Poll::Ready(Some(self.inputs[self.pos].clone()));
        self.pos += 1;
        poll
    }
}

impl ResettableStream for VecStream {
    async fn reset(&mut self, number: u64) {
        tracing::info!(number, "reset");
        self.pos = self
            .inputs
            .iter()
            .position(|input| input.block.number == number + 1)
            .unwrap_or_else(|| panic!("cannot reset to unknown block height {number}"));
        if let Some(reorg) = self.reorg.take() {
            self.inputs = reorg;
        }
    }
}

/// Generate a block ID for testing.
pub(crate) fn block_id(number: u64) -> L1BlockId {
    let parent = keccak256(number.saturating_sub(1).to_le_bytes());
    let hash = keccak256(number.to_le_bytes());
    L1BlockId {
        number,
        hash,
        parent,
    }
}

/// Generate an arbitrary node for testing.
pub(crate) fn make_node(i: usize) -> NodeSetEntry {
    let address = Address::random();
    let staking_key = TaggedBase64::new("KEY", &i.to_le_bytes()).unwrap();
    NodeSetEntry {
        address,
        staking_key,
        stake: i.try_into().unwrap(),
        commission: Ratio::new(5, 100),
    }
}

/// Generate an empty wallet snapshot for testing.
pub(crate) fn empty_wallet(l1_block: L1BlockInfo) -> WalletSnapshot {
    WalletSnapshot {
        l1_block,
        nodes: Default::default(),
        pending_exits: Default::default(),
        pending_undelegations: Default::default(),
        claimed_rewards: Default::default(),
    }
}

/// Generate a random L1 input for testing.
pub(crate) fn random_block_input(number: u64) -> BlockInput {
    // TODO populate random events
    BlockInput::empty(number)
}

impl BlockInput {
    /// Create a [`BlockInput`] with no events, just L1 block information.
    pub(crate) fn empty(number: u64) -> BlockInput {
        Self {
            block: block_id(number),
            finalized: block_id(0),
            timestamp: 12 * number,
            events: vec![],
        }
    }
}

impl<S: Default> super::State<S> {
    pub(crate) fn with_l1_block_range(start: u64, end: u64) -> Self {
        let blocks = (start..end).map(BlockData::empty).collect::<Vec<_>>();
        let blocks_by_hash = blocks
            .iter()
            .map(|block| (block.block.hash, block.block.number))
            .collect();
        Self {
            blocks,
            blocks_by_hash,
            storage: Default::default(),
        }
    }
}

impl BlockData {
    /// Generate a test L1 block with no staking-related data.
    pub(super) fn empty(number: u64) -> Self {
        let block = block_id(number);
        let timestamp = 12 * number;
        Self {
            block,
            timestamp,
            node_set: FullNodeSetSnapshot {
                nodes: Default::default(),
                l1_block: L1BlockInfo {
                    number,
                    hash: block.hash,
                    timestamp,
                },
            },
            node_set_update: Some(Default::default()),
            wallets: Default::default(),
            wallets_update: Some(Default::default()),
        }
    }
}

/// Process events from the given stream until the predicate is satisfied.
///
/// Returns a lock on the state, frozen after the first observation where the predicate was
/// satisfied.
pub(crate) async fn subscribe_until<S>(
    state: &'_ Arc<RwLock<State<S>>>,
    stream: impl ResettableStream + Send + 'static,
    p: impl Fn(&State<S>) -> bool,
) -> RwLockReadGuard<'_, State<S>>
where
    S: L1Persistence + Sync + 'static,
{
    let task = spawn(State::subscribe(state.clone(), stream));

    // Wait for the predicate.
    loop {
        sleep(Duration::from_millis(100)).await;
        let state = state.read().await;
        if p(&state) {
            task.abort();
            let _ = task.await;
            break state;
        }
    }
}
