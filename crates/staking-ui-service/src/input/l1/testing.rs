#![cfg(any(test, feature = "testing"))]

use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use crate::types::common::NodeSetEntry;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, FixedBytes, U256, keccak256},
    providers::{
        Identity, ProviderBuilder, WalletProvider,
        ext::AnvilApi,
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            SimpleNonceManager, WalletFiller,
        },
    },
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol_types::SolValue,
};
use async_lock::RwLockReadGuard;
use espresso_contract_deployer::{
    Contract, Contracts, build_signer, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table,
};
use hotshot_contract_adapter::{
    sol_types::{
        G1PointSol,
        RewardClaim::RewardsClaimed,
        StakeTableV2::{
            self, ConsensusKeysUpdatedV2, Delegated, ExitEscrowPeriodUpdated, Undelegated,
            ValidatorExit, ValidatorRegistered, ValidatorRegisteredV2, Withdrawal,
        },
    },
    stake_table::{StateSignatureSol, sign_address_bls, sign_address_schnorr},
};
use hotshot_state_prover::v1::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;
use hotshot_types::{
    light_client::{StateKeyPair, StateVerKey, hash_bytes_to_field},
    signature_key::BLSKeyPair,
    traits::signature_key::{SignatureKey, StateSignatureKey},
};
use jf_signature::{SignatureScheme, schnorr::SchnorrSignatureScheme};
use pretty_assertions::assert_eq;
use rand::{CryptoRng, Rng, RngCore, SeedableRng, rngs::StdRng, seq::IteratorRandom};
use staking_cli::demo::{DelegationConfig, StakingTransactions};
use tide_disco::{Error as _, StatusCode, Url};
use tokio::{task::spawn, time::sleep};

use super::*;

/// Easy-setup storage that just uses memory.
#[derive(Clone, Debug, Default)]
pub struct MemoryStorage {
    snapshot: Arc<RwLock<Option<Snapshot>>>,
}

impl L1Persistence for MemoryStorage {
    async fn finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        Ok(self.snapshot.read().await.clone())
    }

    async fn save_genesis(&mut self, snapshot: Snapshot) -> Result<()> {
        *self.snapshot.write().await = Some(snapshot);
        Ok(())
    }

    async fn apply_updates(&mut self, updates: Vec<Update>) -> Result<()> {
        let mut lock = self.snapshot.write().await;
        let snapshot = lock.get_or_insert(Snapshot::empty(block_snapshot(0)));

        for update in updates {
            for diff in update.node_set_diffs {
                snapshot.node_set.apply(&diff);
            }
            for (address, diffs) in update.wallet_diffs {
                for diff in diffs {
                    snapshot.wallets.apply(address, &diff);
                }
            }
            snapshot.block = update.block;
        }
        Ok(())
    }
}

/// Storage that always fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct FailStorage;

impl L1Persistence for FailStorage {
    async fn finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }

    async fn save_genesis(&mut self, _snapshot: Snapshot) -> Result<()> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }

    async fn apply_updates(&mut self, _updates: Vec<Update>) -> Result<()> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }
}

/// Resettable stream that yields a predefined list of inputs.
#[derive(Clone, Debug)]
pub struct VecStream {
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
    pub fn infinite() -> Self {
        Self {
            panic_at_end: false,
            ..Default::default()
        }
    }

    /// Append a new L1 block input to be yielded by the stream.
    pub fn push(&mut self, input: BlockInput) {
        self.inputs.push(input);
    }

    /// Provide an alternative sequence of inputs to yield after the stream is reset.
    pub fn with_reorg(mut self, inputs: Vec<BlockInput>) -> Self {
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

/// [`L1Catchup`] implementation which never does catchup.
///
/// In other words, this provider always pretends that the given starting block is the latest block.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCatchup;

impl L1Catchup for NoCatchup {
    async fn fast_forward(
        &self,
        _from: u64,
    ) -> Result<BTreeMap<L1BlockId, (Timestamp, Vec<L1Event>)>> {
        Ok(Default::default())
    }
}

/// [`L1Catchup`] implementation which supplies a predefined event sequence.
#[derive(Clone, Debug)]
pub struct CatchupFromEvents {
    events: BTreeMap<L1BlockId, (Timestamp, Vec<L1Event>)>,
}

impl CatchupFromEvents {
    pub fn from_blocks(blocks: impl IntoIterator<Item = BlockInput>) -> Self {
        Self {
            events: blocks
                .into_iter()
                .map(|input| (input.block, (input.timestamp, input.events)))
                .collect(),
        }
    }
}

impl L1Catchup for CatchupFromEvents {
    async fn fast_forward(
        &self,
        from: u64,
    ) -> Result<BTreeMap<L1BlockId, (Timestamp, Vec<L1Event>)>> {
        Ok(self
            .events
            .clone()
            .into_iter()
            .skip_while(|(id, ..)| id.number <= from)
            .collect())
    }
}

/// Generate a block ID for testing.
pub fn block_id(number: u64) -> L1BlockId {
    let parent = keccak256(number.saturating_sub(1).to_le_bytes());
    let hash = keccak256(number.to_le_bytes());
    L1BlockId {
        number,
        hash,
        parent,
    }
}

/// Generate a block snapshot for testing.
pub fn block_snapshot(number: u64) -> L1BlockSnapshot {
    L1BlockSnapshot {
        id: block_id(number),
        timestamp: 12 * number,
        exit_escrow_period: 3600,
    }
}

/// Generate an arbitrary node for testing.
pub fn make_node(i: usize) -> NodeSetEntry {
    (&validator_registered_event(StdRng::seed_from_u64(i as u64))).into()
}

/// Generate random L1 events for testing.
///
/// The generation process is stateful, which makes it possible to generate random events such that
/// every generated event is "valid" given the events that came before it (e.g. no duplicate
/// registrations, no delegations to an unregistered validator, etc.).
#[derive(Debug)]
pub struct EventGenerator {
    nodes: HashSet<Address>,
    delegations: HashMap<(Address, Address), U256>,
    pending_undelegations: HashMap<(Address, Address), U256>,
    pending_exits: HashMap<(Address, Address), U256>,
    exited_nodes: HashSet<Address>,
    rng: StdRng,
    stake_table_only: bool,
}

impl Default for EventGenerator {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            delegations: Default::default(),
            pending_undelegations: Default::default(),
            pending_exits: Default::default(),
            exited_nodes: Default::default(),
            rng: StdRng::from_seed(Default::default()),
            stake_table_only: false,
        }
    }
}

impl EventGenerator {
    /// Generate only events which are relevant to [`StakeTableState`].
    pub fn stake_table_events() -> Self {
        Self {
            stake_table_only: true,
            ..Default::default()
        }
    }
}

impl Iterator for EventGenerator {
    type Item = L1Event;

    fn next(&mut self) -> Option<Self::Item> {
        // Generate a random event until we get one that is possible given the current state.
        loop {
            // Assign each type of event a numeric code. We put stake table events first so we can
            // easily choose to generate only stake table events.
            const REGISTER: usize = 0;
            const REGISTER_V2: usize = 1;
            const DEREGISTER: usize = 2;
            const DELEGATE: usize = 3;
            const UNDELEGATE: usize = 4;
            const WITHDRAWAL: usize = 5;
            const KEY_UPDATE: usize = 6;

            const MAX_STAKE_TABLE_EVENT_TYPE: usize = 7;

            // Other contract events that the UI service cares about but consensus does not.
            const EXIT_ESCROW_PERIOD_UPDATED: usize = 7;

            const CLAIM_REWARDS: usize = 8;

            const MAX_EVENT_TYPE: usize = 9;

            // Generate the code for a random event type.
            let max = if self.stake_table_only {
                MAX_STAKE_TABLE_EVENT_TYPE
            } else {
                MAX_EVENT_TYPE
            };
            let event_type = self.rng.gen_range(0..max);

            let event = match event_type {
                t @ (REGISTER | REGISTER_V2) => {
                    let event = validator_registered_event(&mut self.rng);

                    // Insert the node so we can reference it in later events (like delegations).
                    // This insert should always return `true` because with a random address, it is
                    // vanishingly unlikely we have generated this same address before.
                    assert!(self.nodes.insert(event.account));

                    if t == REGISTER {
                        StakeTableV2Events::ValidatorRegistered(ValidatorRegistered {
                            account: event.account,
                            blsVk: event.blsVK,
                            schnorrVk: event.schnorrVK,
                            commission: event.commission,
                        })
                        .into()
                    } else {
                        StakeTableV2Events::ValidatorRegisteredV2(event).into()
                    }
                }

                DEREGISTER => {
                    // Choose a random node to deregister.
                    let Some(&node) = self.nodes.iter().choose(&mut self.rng) else {
                        // If there are none try again.
                        continue;
                    };

                    // Move all delegations to this node to pending exits
                    // and remove from delegations
                    let exits: Vec<_> = self
                        .delegations
                        .iter()
                        .filter(|((_, n), _)| *n == node)
                        .map(|((d, n), amount)| ((*d, *n), *amount))
                        .collect();

                    for (key, amount) in exits {
                        self.delegations.remove(&key);
                        self.pending_exits.insert(key, amount);
                    }

                    self.nodes.remove(&node);
                    self.exited_nodes.insert(node);
                    StakeTableV2Events::ValidatorExit(ValidatorExit { validator: node }).into()
                }

                EXIT_ESCROW_PERIOD_UPDATED => {
                    // Set the exit escrow period to something random.
                    StakeTableV2Events::ExitEscrowPeriodUpdated(ExitEscrowPeriodUpdated {
                        newExitEscrowPeriod: self.rng.next_u64(),
                    })
                    .into()
                }

                DELEGATE => {
                    // Choose a random validator to delegate to.
                    let Some(&validator) = self.nodes.iter().choose(&mut self.rng) else {
                        continue;
                    };
                    let seed = self.rng.gen_range(1u64..101u64);
                    let mut address_bytes = FixedBytes::<32>::default();
                    address_bytes[..8].copy_from_slice(&seed.to_le_bytes());
                    let delegator = Address::from_word(address_bytes);
                    let amount = U256::from(self.rng.gen_range(100u64..10000u64));
                    let key = (delegator, validator);

                    self.delegations
                        .entry(key)
                        .and_modify(|existing| *existing += amount)
                        .or_insert(amount);

                    StakeTableV2Events::Delegated(Delegated {
                        delegator,
                        validator,
                        amount,
                    })
                    .into()
                }

                UNDELEGATE => {
                    if self.delegations.is_empty() {
                        continue;
                    };

                    let delegations: Vec<_> = self
                        .delegations
                        .iter()
                        .filter(|(key, _)| !self.pending_undelegations.contains_key(key))
                        .collect();

                    if delegations.is_empty() {
                        continue;
                    }

                    let (key, amount) = delegations
                        .iter()
                        .choose(&mut self.rng)
                        .map(|(k, v)| (**k, **v))
                        .unwrap();

                    let (delegator, node) = key;

                    let undelegate_amount =
                        (amount * U256::from(self.rng.gen_range(1u64..50u64))) / U256::from(100);

                    let remaining = amount - undelegate_amount;

                    // Update or remove delegation based on remaining amount
                    if remaining.is_zero() {
                        self.delegations.remove(&key);
                    } else {
                        self.delegations.insert(key, remaining);
                    }

                    self.pending_undelegations.insert(key, undelegate_amount);

                    StakeTableV2Events::Undelegated(Undelegated {
                        delegator,
                        validator: node,
                        amount: undelegate_amount,
                    })
                    .into()
                }

                WITHDRAWAL => {
                    let pending_undelegations: Vec<_> =
                        self.pending_undelegations.keys().copied().collect();
                    let pending_exits: Vec<_> = self.pending_exits.keys().copied().collect();

                    let total = pending_undelegations.len() + pending_exits.len();
                    if total == 0 {
                        continue;
                    }

                    // Choose a random withdrawal and remove it
                    let idx = self.rng.gen_range(0..total);
                    let (account, _node, amount) = if idx < pending_undelegations.len() {
                        let key = pending_undelegations[idx];
                        let amount = self.pending_undelegations.remove(&key).unwrap();
                        (key.0, key.1, amount)
                    } else {
                        let key = pending_exits[idx - pending_undelegations.len()];
                        let amount = self.pending_exits.remove(&key).unwrap();
                        (key.0, key.1, amount)
                    };

                    StakeTableV2Events::Withdrawal(Withdrawal { account, amount }).into()
                }

                KEY_UPDATE => {
                    let Some(&node) = self.nodes.iter().choose(&mut self.rng) else {
                        continue;
                    };

                    let mut new_seed = [0u8; 32];
                    self.rng.fill_bytes(&mut new_seed);
                    let index = self.rng.next_u64();

                    let new_bls_key = BLSKeyPair::generate(&mut StdRng::from_seed(new_seed));
                    let new_schnorr_key = StateKeyPair::generate_from_seed_indexed(new_seed, index);

                    let bls_sig: G1PointSol = sign_address_bls(&new_bls_key, node).into();
                    let schnorr_sig: StateSignatureSol =
                        sign_address_schnorr(&new_schnorr_key, node).into();

                    StakeTableV2Events::ConsensusKeysUpdatedV2(ConsensusKeysUpdatedV2 {
                        account: node,
                        blsVK: new_bls_key.ver_key().into(),
                        schnorrVK: new_schnorr_key.ver_key().into(),
                        blsSig: bls_sig.into(),
                        schnorrSig: schnorr_sig.into(),
                    })
                    .into()
                }

                CLAIM_REWARDS => {
                    let delegators: Vec<Address> = self
                        .delegations
                        .keys()
                        .map(|(d, _)| *d)
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect();

                    if delegators.is_empty() {
                        continue;
                    }
                    let user = *delegators.iter().choose(&mut self.rng).unwrap();
                    let amount = U256::from(self.rng.gen_range(10u64..1000u64));

                    RewardClaimEvents::RewardsClaimed(RewardsClaimed { user, amount }).into()
                }

                _ => unreachable!(),
            };
            return Some(event);
        }
    }
}

/// Generate a valid [`ValidatorRegisteredV2`] event.
pub fn validator_registered_event(mut rng: impl RngCore + CryptoRng) -> ValidatorRegisteredV2 {
    let mut address_bytes = FixedBytes::<32>::default();
    rng.fill_bytes(address_bytes.as_mut_slice());
    let account = Address::from_word(address_bytes);
    validator_registered_event_with_account(rng, account)
}

/// Generate a valid [`ValidatorRegisteredV2`] event with the given [`Address`].
pub fn validator_registered_event_with_account(
    mut rng: impl RngCore + CryptoRng,
    account: Address,
) -> ValidatorRegisteredV2 {
    let index = rng.next_u64();
    let (bls_vk, bls_sk) = PubKey::generated_from_seed_indexed(Default::default(), index);
    let (schnorr_vk, schnorr_sk) =
        StateVerKey::generated_from_seed_indexed(Default::default(), index);

    let auth_msg = account.abi_encode();
    let bls_sig = PubKey::sign(&bls_sk, &auth_msg).unwrap();
    let schnorr_sig = SchnorrSignatureScheme::sign(
        &(),
        &schnorr_sk,
        [hash_bytes_to_field(&auth_msg).unwrap()],
        &mut rng,
    )
    .unwrap();

    let commission = rng.gen_range(0..COMMISSION_BASIS_POINTS);

    ValidatorRegisteredV2 {
        account,
        blsVK: bls_vk.into(),
        schnorrVK: schnorr_vk.into(),
        commission,
        blsSig: G1PointSol::from(bls_sig).into(),
        schnorrSig: StateSignatureSol::from(schnorr_sig).into(),
        metadataUri: "https://example.com/validator-metadata.json".to_string(),
    }
}

/// Generate random L1 inputs for testing.
///
/// The generation process is stateful, which makes it possible to generate random events in a
/// sequence of L1 blocks such that every generated event is "valid" given the events that came
/// before it (e.g. no duplicate registrations, no delegations to an unregistered validator, etc.).
#[derive(derive_more::Debug)]
pub struct InputGenerator {
    #[debug("Iterator")]
    events: Box<dyn Iterator<Item = L1Event>>,
    next_block: Option<u64>,
    rng: StdRng,
}

impl Default for InputGenerator {
    fn default() -> Self {
        Self::from_events(EventGenerator::default())
    }
}

impl InputGenerator {
    /// Generate inputs from a given sequence of events.
    pub fn from_events(events: impl IntoIterator<Item = L1Event> + 'static) -> Self {
        Self {
            events: Box::new(events.into_iter()),
            next_block: Some(0),
            rng: StdRng::from_seed(Default::default()),
        }
    }

    /// A generator like `self`, but the next block yielded will have number `from`.
    pub fn from_block(mut self, from: u64) -> Self {
        self.next_block = Some(from);
        self
    }
}

impl Iterator for InputGenerator {
    type Item = BlockInput;

    fn next(&mut self) -> Option<Self::Item> {
        // Select a realistic random number of events to include in this block.
        let n_events = self.rng.gen_range(0..5);

        // Generate block.
        let mut input = BlockInput::empty(self.next_block?);
        for _ in 0..n_events {
            let Some(event) = self.events.next() else {
                // If we run out of events, finish the current block, to ensure we use all the
                // events that did get yielded. But then stop producing more blocks.
                self.next_block = None;
                break;
            };
            input.events.push(event);
        }
        self.next_block = self.next_block.map(|b| b + 1);
        Some(input)
    }
}

impl BlockInput {
    /// Create a [`BlockInput`] with no events, just L1 block information.
    pub fn empty(number: u64) -> BlockInput {
        let block = block_snapshot(number);
        Self {
            block: block.id(),
            timestamp: block.timestamp(),
            finalized: block_id(0),
            events: vec![],
        }
    }

    /// A [`BlockInput`] like `self` but with `event` added.
    pub fn with_event(mut self, event: impl Into<L1Event>) -> Self {
        self.events.push(event.into());
        self
    }
}

impl<S: Default> super::State<S> {
    pub fn with_l1_block_range(start: u64, end: u64) -> Self {
        let blocks = (start..end).map(BlockData::empty).collect::<Vec<_>>();
        let blocks_by_hash = blocks
            .iter()
            .map(|block| (block.block().hash(), block.block().number()))
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
    pub fn empty(number: u64) -> Self {
        Self {
            state: Snapshot::empty(block_snapshot(number)),
            node_set_update: Some(Default::default()),
            wallets_update: Some(Default::default()),
        }
    }
}

/// Process events from the given stream until the predicate is satisfied.
///
/// Returns a lock on the state, frozen after the first observation where the predicate was
/// satisfied.
pub async fn subscribe_until<S>(
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

const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub mnemonic: String,
    pub deployer_index: u32,
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
    pub exit_escrow_period_secs: u64,
    pub token_name: String,
    pub token_symbol: String,
    pub initial_token_supply: u64,
    pub ops_timelock_delay_secs: u64,
    pub safe_exit_timelock_delay_secs: u64,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            mnemonic: DEV_MNEMONIC.to_string(),
            deployer_index: 0,
            blocks_per_epoch: 100,
            epoch_start_block: 1,
            exit_escrow_period_secs: 90,
            token_name: "Espresso".to_string(),
            token_symbol: "ESP".to_string(),
            initial_token_supply: 3_590_000_000,
            ops_timelock_delay_secs: 0,
            safe_exit_timelock_delay_secs: 10,
        }
    }
}

pub struct ContractDeployment {
    pub rpc_url: Url,
    pub stake_table_addr: Address,
    pub reward_claim_addr: Address,
    pub token_addr: Address,
}

impl ContractDeployment {
    pub async fn deploy(rpc_url: Url) -> Result<Self> {
        Self::deploy_with_config(rpc_url, DeploymentConfig::default()).await
    }

    pub async fn deploy_with_config(rpc_url: Url, config: DeploymentConfig) -> Result<Self> {
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(build_signer(
                &config.mnemonic,
                config.deployer_index,
            )))
            .connect_http(rpc_url.clone());

        let deployer_address = provider.default_signer_address();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &Default::default(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let args = DeployerArgsBuilder::default()
            .deployer(provider.clone())
            .rpc_url(rpc_url.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(config.blocks_per_epoch)
            .epoch_start_block(config.epoch_start_block)
            .multisig_pauser(deployer_address)
            .exit_escrow_period(U256::from(config.exit_escrow_period_secs))
            .token_name(config.token_name)
            .token_symbol(config.token_symbol)
            .initial_token_supply(U256::from(config.initial_token_supply))
            .ops_timelock_delay(U256::from(config.ops_timelock_delay_secs))
            .ops_timelock_admin(deployer_address)
            .ops_timelock_proposers(vec![deployer_address])
            .ops_timelock_executors(vec![deployer_address])
            .safe_exit_timelock_delay(U256::from(config.safe_exit_timelock_delay_secs))
            .safe_exit_timelock_admin(deployer_address)
            .safe_exit_timelock_proposers(vec![deployer_address])
            .safe_exit_timelock_executors(vec![deployer_address])
            .use_timelock_owner(false)
            .build()
            .map_err(|err| {
                Error::internal().context(format!("Failed to build deployer args: {err}"))
            })?;

        let mut contracts = Contracts::new();
        args.deploy_all(&mut contracts).await.map_err(|err| {
            Error::internal().context(format!("Failed to deploy contracts: {err}"))
        })?;

        let stake_table_addr = contracts
            .address(Contract::StakeTableProxy)
            .ok_or_else(|| Error::internal().context("StakeTable address not found"))?;
        let reward_claim_addr = contracts
            .address(Contract::RewardClaimProxy)
            .ok_or_else(|| Error::internal().context("RewardClaim address not found"))?;
        let token_addr = contracts
            .address(Contract::EspTokenProxy)
            .ok_or_else(|| Error::internal().context("Token address not found"))?;

        tracing::info!("Deployed contracts:");
        tracing::info!("StakeTable: {stake_table_addr}");
        tracing::info!("RewardClaim: {reward_claim_addr}");
        tracing::info!("Token: {token_addr}");

        Ok(Self {
            rpc_url,
            stake_table_addr,
            reward_claim_addr,
            token_addr,
        })
    }

    pub fn create_test_validators(
        count: usize,
    ) -> Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)> {
        (0..count)
            .map(|i| {
                let index = i as u32;
                let seed = [index as u8; 32];
                let signer = build_signer(DEV_MNEMONIC, index);
                let bls_key_pair = BLSKeyPair::generate(&mut StdRng::from_seed(seed));
                let state_key_pair = StateKeyPair::generate_from_seed_indexed(seed, index as u64);
                (signer, bls_key_pair, state_key_pair)
            })
            .collect()
    }

    pub async fn register_validators(
        &self,
        validators: Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)>,
        delegation_config: DelegationConfig,
    ) -> Result<Vec<TransactionReceipt>> {
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(build_signer(DEV_MNEMONIC, 0)))
            .connect_http(self.rpc_url.clone());

        let mut staking_txns = StakingTransactions::create(
            self.rpc_url.clone(),
            &provider,
            self.stake_table_addr,
            validators,
            None,
            delegation_config,
        )
        .await
        .map_err(|err| {
            Error::internal().context(format!("Failed to create staking transactions: {err}"))
        })?;

        let receipts = staking_txns.apply_all().await.map_err(|err| {
            Error::internal().context(format!("Failed to apply transactions: {err}"))
        })?;

        Ok(receipts)
    }

    pub fn spawn_task(&self) -> BackgroundStakeTableOps {
        BackgroundStakeTableOps::spawn(
            self.rpc_url.clone(),
            self.stake_table_addr,
            Arc::new(false.into()),
            None,
        )
    }

    pub fn spawn_task_with_interval(&self, interval: Duration) -> BackgroundStakeTableOps {
        BackgroundStakeTableOps::spawn(
            self.rpc_url.clone(),
            self.stake_table_addr,
            Arc::new(false.into()),
            Some(interval),
        )
    }
}

/// Spawns a background task that continuously performs random stake table operations.
/// Operations include: registering validators, updating consensus keys, undelegating, and deregistering.
/// Used in tests to have some activity on L1 and validate events fetching
pub struct BackgroundStakeTableOps {
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

type SimpleNonceProvider = FillProvider<
    JoinFill<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            NonceFiller<SimpleNonceManager>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    alloy::providers::RootProvider,
>;

struct DelegatorInfo {
    address: Address,
    provider: SimpleNonceProvider,
}

struct ValidatorInfo {
    index: u64,
    address: Address,
    provider: SimpleNonceProvider,
    delegators: Vec<DelegatorInfo>,
}

struct BackgroundTaskState {
    rpc_url: Url,
    stake_table_addr: Address,
    validator_index: u64,
    registered_validators: Vec<ValidatorInfo>,
    pending_undelegations: HashSet<(Address, Address)>,
    pending_exits: HashSet<(Address, Address)>,
    delegator_providers: HashMap<Address, SimpleNonceProvider>,
    rng: StdRng,
}

impl BackgroundTaskState {
    fn new(rpc_url: Url, stake_table_addr: Address) -> Self {
        Self {
            rpc_url,
            stake_table_addr,
            validator_index: 0,
            registered_validators: Vec::new(),
            pending_undelegations: HashSet::new(),
            pending_exits: HashSet::new(),
            delegator_providers: HashMap::new(),
            rng: StdRng::from_entropy(),
        }
    }

    async fn run(&mut self, operation: usize) {
        match operation {
            0 => self.register_validator().await,
            1 => self.update_consensus_keys().await,
            2 => self.undelegate().await,
            3 => self.deregister_validator().await,
            4 => self.withdrawal().await,
            _ => unreachable!(),
        }
    }

    async fn register_validator(&mut self) {
        tracing::debug!("register validator");
        let seed = [self.validator_index as u8; 32];
        let signer = PrivateKeySigner::random();
        let bls_key = BLSKeyPair::generate(&mut StdRng::from_seed(seed));
        let schnorr_key = StateKeyPair::generate_from_seed_indexed(seed, self.validator_index);

        let deployer = build_signer(DEV_MNEMONIC, 0);
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(deployer.clone()))
            .connect_http(self.rpc_url.clone());

        tracing::debug!(signer = %signer.address(), "register");
        match StakingTransactions::create(
            self.rpc_url.clone(),
            &provider,
            self.stake_table_addr,
            vec![(signer.clone(), bls_key.clone(), schnorr_key.clone())],
            None,
            DelegationConfig::MultipleDelegators,
        )
        .await
        {
            Ok(mut staking_txns) => {
                let validator_provider = staking_txns.provider(signer.address()).unwrap().clone();

                let delegator_infos: Vec<DelegatorInfo> = staking_txns
                    .delegations()
                    .iter()
                    .filter(|d| d.validator == signer.address())
                    .filter_map(|d| {
                        let provider = staking_txns.provider(d.from)?.clone();
                        let provider = ProviderBuilder::new()
                            .with_simple_nonce_management()
                            .wallet(provider.wallet().clone())
                            .connect_http(self.rpc_url.clone());
                        Some(DelegatorInfo {
                            address: d.from,
                            provider,
                        })
                    })
                    .collect();

                match staking_txns.apply_all().await {
                    Ok(_) => {
                        tracing::info!(
                            "Background: Registered validator #{} with {} delegators",
                            self.validator_index,
                            delegator_infos.len(),
                        );

                        for delegator_info in &delegator_infos {
                            self.delegator_providers
                                .insert(delegator_info.address, delegator_info.provider.clone());
                        }

                        let provider = ProviderBuilder::new()
                            .with_simple_nonce_management()
                            .wallet(validator_provider.wallet().clone())
                            .connect_http(self.rpc_url.clone());
                        self.registered_validators.push(ValidatorInfo {
                            index: self.validator_index,
                            address: signer.address(),
                            provider,
                            delegators: delegator_infos,
                        });
                    }
                    Err(err) => {
                        tracing::error!(
                            "Background: Failed to register validator #{}: {err:#}",
                            self.validator_index
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Background: Failed to create staking transactions for validator #{}: {e:?}",
                    self.validator_index
                );

                let _ = provider
                    .anvil_set_balance(
                        provider.default_signer_address(),
                        U256::from(100_000) * U256::from(10).pow(U256::from(18)),
                    ) // 100k ETH)
                    .await;
            }
        }
        self.validator_index += 1;
    }

    async fn update_consensus_keys(&mut self) {
        if self.registered_validators.is_empty() {
            return;
        }
        tracing::debug!("update consensus keys");

        let idx = self.rng.gen_range(0..self.registered_validators.len());
        let validator = &self.registered_validators[idx];

        let mut new_seed = [0u8; 32];
        self.rng.fill_bytes(&mut new_seed);

        let new_bls_key = BLSKeyPair::generate(&mut StdRng::from_seed(new_seed));
        let new_schnorr_key = StateKeyPair::generate_from_seed_indexed(new_seed, validator.index);

        let bls_sig: G1PointSol = sign_address_bls(&new_bls_key, validator.address).into();
        let schnorr_sig: StateSignatureSol =
            sign_address_schnorr(&new_schnorr_key, validator.address).into();

        tracing::debug!(signer = %validator.provider.default_signer_address(), "update keys");
        match StakeTableV2::new(self.stake_table_addr, &validator.provider)
            .updateConsensusKeysV2(
                new_bls_key.ver_key().into(),
                new_schnorr_key.ver_key().into(),
                bls_sig.into(),
                schnorr_sig.into(),
            )
            .send()
            .await
        {
            Ok(pending) => match pending.get_receipt().await {
                Ok(receipt) => tracing::info!(
                    "Background: Updated keys for validator #{} (tx: {:?})",
                    validator.index,
                    receipt.transaction_hash
                ),
                Err(e) => tracing::error!(
                    "Background: Failed to get receipt for validator #{}: {e:?}",
                    validator.index
                ),
            },
            Err(e) => tracing::error!(
                "Background: Failed to update keys for validator #{}: {e:?}",
                validator.index
            ),
        }
    }

    async fn undelegate(&mut self) {
        if self.registered_validators.is_empty() {
            return;
        }
        tracing::debug!("undelegate");

        let val_idx = self.rng.gen_range(0..self.registered_validators.len());
        let validator = &self.registered_validators[val_idx];

        if validator.delegators.is_empty() {
            return;
        }

        let del_idx = self.rng.gen_range(0..validator.delegators.len());
        let delegator = &validator.delegators[del_idx];
        let key = (validator.address, delegator.address);

        if self.pending_undelegations.contains(&key) {
            return;
        }

        let provider = ProviderBuilder::new().connect_http(self.rpc_url.clone());
        let stake_table = StakeTableV2::new(self.stake_table_addr, &provider);

        if let Ok(delegated_amount) = stake_table
            .delegations(validator.address, delegator.address)
            .call()
            .await
            && delegated_amount > U256::ZERO
        {
            let undelegate_amount = delegated_amount / U256::from(2);

            tracing::debug!(
                signer = %delegator.provider.default_signer_address(),
                "undelegate"
            );
            match StakeTableV2::new(self.stake_table_addr, &delegator.provider)
                .undelegate(validator.address, undelegate_amount)
                .send()
                .await
            {
                Ok(pending_tx) => match pending_tx.get_receipt().await {
                    Ok(receipt) => {
                        tracing::info!(
                            "Background: Undelegated {} ESP from validator #{} by delegator {:?} (tx: {:?})",
                            undelegate_amount / U256::from(10).pow(U256::from(18)),
                            validator.index,
                            delegator.address,
                            receipt.transaction_hash
                        );
                        self.pending_undelegations.insert(key);
                    }
                    Err(e) => tracing::error!(
                        "Background: Failed to get undelegate receipt for validator #{}: {e:?}",
                        validator.index
                    ),
                },
                Err(e) => tracing::error!(
                    "Background: Failed to undelegate from validator #{}: {e:?}",
                    validator.index
                ),
            }
        }
    }

    async fn deregister_validator(&mut self) {
        if self.registered_validators.is_empty() {
            return;
        }
        tracing::debug!("deregister validator");

        let idx = self.rng.gen_range(0..self.registered_validators.len());
        let validator = &self.registered_validators[idx];

        tracing::debug!(signer = %validator.provider.default_signer_address(), "deregister");
        match StakeTableV2::new(self.stake_table_addr, &validator.provider)
            .deregisterValidator()
            .send()
            .await
        {
            Ok(pending) => match pending.get_receipt().await {
                Ok(_) => {
                    tracing::info!("Background: Validator #{} deregistered", validator.index);

                    for delegator in &validator.delegators {
                        let key = (validator.address, delegator.address);
                        self.pending_exits.insert(key);
                    }

                    self.registered_validators.remove(idx);
                }
                Err(e) => tracing::error!(
                    "Background: Failed to get deregister receipt for validator #{}: {e:?}",
                    validator.index
                ),
            },
            Err(e) => {
                tracing::error!(
                    "Background: Failed to deregister validator #{}: {e:?}",
                    validator.index
                );
            }
        }
    }

    async fn withdrawal(&mut self) {
        let total_pending = self.pending_undelegations.len() + self.pending_exits.len();
        if total_pending == 0 {
            return;
        }
        tracing::debug!("withdraw");

        let idx = self.rng.gen_range(0..total_pending);
        let (key, is_exit) = if idx < self.pending_undelegations.len() {
            (*self.pending_undelegations.iter().nth(idx).unwrap(), false)
        } else {
            (
                *self
                    .pending_exits
                    .iter()
                    .nth(idx - self.pending_undelegations.len())
                    .unwrap(),
                true,
            )
        };

        let (validator_addr, delegator_addr) = key;

        if let Some(provider) = self.delegator_providers.get(&delegator_addr) {
            tracing::debug!(signer = %provider.default_signer_address(), "withdraw");
            let result = if is_exit {
                StakeTableV2::new(self.stake_table_addr, &provider)
                    .claimValidatorExit(validator_addr)
                    .send()
                    .await
            } else {
                StakeTableV2::new(self.stake_table_addr, &provider)
                    .claimWithdrawal(validator_addr)
                    .send()
                    .await
            };

            match result {
                Ok(pending_tx) => match pending_tx.get_receipt().await {
                    Ok(receipt) => {
                        tracing::info!(
                            "Background: Withdrawal from {:?} (tx: {:?})",
                            validator_addr,
                            receipt.transaction_hash
                        );
                        if is_exit {
                            self.pending_exits.remove(&key);
                        } else {
                            self.pending_undelegations.remove(&key);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Background: Failed to get withdrawal receipt: {e:?}")
                    }
                },
                Err(e) => {
                    // if exit escrow period is not over
                    if !e.to_string().contains("0x5a774357") {
                        tracing::error!("Background: Withdrawal failed: {e:?}");
                    }
                }
            }
        }
    }
}

impl BackgroundStakeTableOps {
    pub fn spawn(
        rpc_url: Url,
        stake_table_addr: Address,
        cancel: Arc<AtomicBool>,
        interval: Option<Duration>,
    ) -> Self {
        let interval = interval.unwrap_or(Duration::from_millis(500));

        let task_handle = tokio::spawn(async move {
            let mut state = BackgroundTaskState::new(rpc_url, stake_table_addr);

            while !cancel.load(Ordering::SeqCst) {
                let event = state.rng.gen_range(0..5);
                state.run(event).await;
                sleep(interval).await;
            }
            tracing::info!("background stake table ops exiting");
        });

        Self {
            task_handle: Some(task_handle),
        }
    }

    pub async fn join(mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.await.ok();
        }
    }
}
impl Drop for BackgroundStakeTableOps {
    fn drop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

pub fn assert_events_eq(l: &L1Event, r: &L1Event) {
    // Workaround for events not implementing [`PartialEq`].
    match (l, r) {
        (L1Event::StakeTable(l), L1Event::StakeTable(r)) => match (l.as_ref(), r.as_ref()) {
            (
                StakeTableV2Events::ValidatorRegistered(l),
                StakeTableV2Events::ValidatorRegistered(r),
            ) => {
                assert_eq!(l.account, r.account);
                assert_eq!(l.blsVk, r.blsVk);
                assert_eq!(l.schnorrVk, r.schnorrVk);
                assert_eq!(l.commission, r.commission);
            }
            (
                StakeTableV2Events::ValidatorRegisteredV2(l),
                StakeTableV2Events::ValidatorRegisteredV2(r),
            ) => {
                assert_eq!(l.account, r.account);
                assert_eq!(l.blsVK, r.blsVK);
                assert_eq!(l.schnorrVK, r.schnorrVK);
                assert_eq!(l.commission, r.commission);
            }
            (StakeTableV2Events::ValidatorExit(l), StakeTableV2Events::ValidatorExit(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::ValidatorExitV2(l), StakeTableV2Events::ValidatorExitV2(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Delegated(l), StakeTableV2Events::Delegated(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Undelegated(l), StakeTableV2Events::Undelegated(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::UndelegatedV2(l), StakeTableV2Events::UndelegatedV2(r)) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::ConsensusKeysUpdated(l),
                StakeTableV2Events::ConsensusKeysUpdated(r),
            ) => {
                assert_eq!(l.account, r.account);
                assert_eq!(l.blsVK, r.blsVK);
                assert_eq!(l.schnorrVK, r.schnorrVK);
            }
            (
                StakeTableV2Events::ConsensusKeysUpdatedV2(l),
                StakeTableV2Events::ConsensusKeysUpdatedV2(r),
            ) => {
                assert_eq!(l.account, r.account);
                assert_eq!(l.blsVK, r.blsVK);
                assert_eq!(l.schnorrVK, r.schnorrVK);
            }
            (
                StakeTableV2Events::CommissionUpdated(l),
                StakeTableV2Events::CommissionUpdated(r),
            ) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::ExitEscrowPeriodUpdated(l),
                StakeTableV2Events::ExitEscrowPeriodUpdated(r),
            ) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::MaxCommissionIncreaseUpdated(l),
                StakeTableV2Events::MaxCommissionIncreaseUpdated(r),
            ) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::MinCommissionUpdateIntervalUpdated(l),
                StakeTableV2Events::MinCommissionUpdateIntervalUpdated(r),
            ) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::OwnershipTransferred(l),
                StakeTableV2Events::OwnershipTransferred(r),
            ) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Paused(l), StakeTableV2Events::Paused(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Unpaused(l), StakeTableV2Events::Unpaused(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Initialized(l), StakeTableV2Events::Initialized(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::RoleAdminChanged(l), StakeTableV2Events::RoleAdminChanged(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::RoleGranted(l), StakeTableV2Events::RoleGranted(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::RoleRevoked(l), StakeTableV2Events::RoleRevoked(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Upgraded(l), StakeTableV2Events::Upgraded(r)) => {
                assert_eq!(l, r);
            }
            (StakeTableV2Events::Withdrawal(l), StakeTableV2Events::Withdrawal(r)) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::WithdrawalClaimed(l),
                StakeTableV2Events::WithdrawalClaimed(r),
            ) => {
                assert_eq!(l, r);
            }
            (
                StakeTableV2Events::ValidatorExitClaimed(l),
                StakeTableV2Events::ValidatorExitClaimed(r),
            ) => {
                assert_eq!(l, r);
            }
            (l, r) => {
                panic!("mismatched stake table events:\n{l:#?}\n{r:#?}",);
            }
        },
        (L1Event::Reward(l), L1Event::Reward(r)) => {
            assert_eq!(l, r);
        }
        (l, r) => {
            panic!("Reward event mismatched with stake table event:\n{l:#?}\n{r:#?}");
        }
    }
}
