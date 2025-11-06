#![cfg(any(test, feature = "testing"))]

use std::{
    collections::HashSet,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use alloy::{
    network::EthereumWallet,
    primitives::{FixedBytes, U256, keccak256},
    providers::{ProviderBuilder, WalletProvider},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol_types::SolValue,
};
use async_lock::RwLockReadGuard;
use espresso_contract_deployer::{
    Contract, Contracts, HttpProviderWithWallet, build_signer, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table,
};
use hotshot_contract_adapter::{
    sol_types::{
        G1PointSol,
        StakeTableV2::{
            self, ExitEscrowPeriodUpdated, ValidatorExit, ValidatorRegistered,
            ValidatorRegisteredV2,
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
use rand::{CryptoRng, Rng, RngCore, SeedableRng, rngs::StdRng, seq::IteratorRandom};
use staking_cli::demo::{DelegationConfig, StakingTransactions};
use tide_disco::{Error as _, StatusCode, Url};
use tokio::{task::spawn, time::sleep};

use crate::types::common::NodeSetEntry;

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

    async fn save_genesis(&self, snapshot: Snapshot) -> Result<()> {
        *self.snapshot.write().await = Some(snapshot);
        Ok(())
    }

    async fn apply_events(
        &self,
        block: L1BlockSnapshot,
        node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> Result<()> {
        let mut lock = self.snapshot.write().await;
        let snapshot = lock.get_or_insert(Snapshot::empty(block_snapshot(0)));

        for diff in node_set_diff {
            snapshot.node_set.apply(&diff);
        }
        for (address, diff) in wallets_diff {
            snapshot.wallets.apply(address, &diff);
        }
        snapshot.block = block;
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

    async fn save_genesis(&self, _snapshot: Snapshot) -> Result<()> {
        Err(Error::catch_all(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FailStorage".into(),
        ))
    }

    async fn apply_events(
        &self,
        _block: L1BlockSnapshot,
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
    rng: StdRng,
    stake_table_only: bool,
}

impl Default for EventGenerator {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
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

            const MAX_STAKE_TABLE_EVENT_TYPE: usize = 3;

            // Other contract events that the UI service cares about but consensus does not.
            const EXIT_ESCROW_PERIOD_UPDATED: usize = 3;

            const MAX_EVENT_TYPE: usize = 4;

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
                    self.nodes.remove(&node);
                    StakeTableV2Events::ValidatorExit(ValidatorExit { validator: node }).into()
                }

                EXIT_ESCROW_PERIOD_UPDATED => {
                    // Set the exit escrow period to something random.
                    StakeTableV2Events::ExitEscrowPeriodUpdated(ExitEscrowPeriodUpdated {
                        newExitEscrowPeriod: self.rng.next_u64(),
                    })
                    .into()
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
    next_block: u64,
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
            next_block: 0,
            rng: StdRng::from_seed(Default::default()),
        }
    }
}

impl Iterator for InputGenerator {
    type Item = BlockInput;

    fn next(&mut self) -> Option<Self::Item> {
        // Select a realistic random number of events to include in this block.
        let n_events = self.rng.gen_range(0..5);

        // Generate block.
        let mut input = BlockInput::empty(self.next_block);
        for _ in 0..n_events {
            input.events.push(self.events.next()?);
        }
        self.next_block += 1;
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
            exit_escrow_period_secs: 250,
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

        println!("Deployed contracts:");
        println!("StakeTable: {stake_table_addr}");
        println!("RewardClaim: {reward_claim_addr}");
        println!("Token: {token_addr}");

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
        BackgroundStakeTableOps::spawn(self.rpc_url.clone(), self.stake_table_addr)
    }
}

/// Spawns a background task that continuously performs random stake table operations.
/// Operations include: registering validators, updating consensus keys, undelegating, and deregistering.
/// Used in tests to have some activity on L1 and validate events fetching
pub struct BackgroundStakeTableOps {
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

struct DelegatorInfo {
    address: Address,
    provider: HttpProviderWithWallet,
}

struct ValidatorInfo {
    index: u64,
    address: Address,
    provider: HttpProviderWithWallet,
    delegators: Vec<DelegatorInfo>,
}

impl BackgroundStakeTableOps {
    pub fn spawn(rpc_url: Url, stake_table_addr: Address) -> Self {
        let task_handle = tokio::spawn(async move {
            let mut validator_index = 0u64;
            let mut registered_validators = Vec::<ValidatorInfo>::new();
            let mut rng = StdRng::from_entropy();

            for i in 0u64.. {
                let operation = rng.gen_range(0..4);

                match operation {
                    0 => {
                        let seed = [validator_index as u8; 32];
                        let signer = build_signer(DEV_MNEMONIC, validator_index as u32);
                        let bls_key = BLSKeyPair::generate(&mut StdRng::from_seed(seed));
                        let schnorr_key =
                            StateKeyPair::generate_from_seed_indexed(seed, validator_index);

                        let provider = ProviderBuilder::new()
                            .wallet(EthereumWallet::from(build_signer(DEV_MNEMONIC, 0)))
                            .connect_http(rpc_url.clone());

                        match StakingTransactions::create(
                            rpc_url.clone(),
                            &provider,
                            stake_table_addr,
                            vec![(signer.clone(), bls_key.clone(), schnorr_key.clone())],
                            DelegationConfig::MultipleDelegators,
                        )
                        .await
                        {
                            Ok(mut staking_txns) => {
                                let validator_provider =
                                    staking_txns.provider(signer.address()).unwrap().clone();

                                let delegator_infos: Vec<DelegatorInfo> = staking_txns
                                    .delegations()
                                    .iter()
                                    .filter(|d| d.validator == signer.address())
                                    .filter_map(|d| {
                                        let provider = staking_txns.provider(d.from)?.clone();

                                        Some(DelegatorInfo {
                                            address: d.from,
                                            provider,
                                        })
                                    })
                                    .collect();

                                if staking_txns.apply_all().await.is_ok() {
                                    println!(
                                        "Background: Registered validator #{validator_index} with {} delegators",
                                        delegator_infos.len(),
                                    );

                                    registered_validators.push(ValidatorInfo {
                                        index: validator_index,
                                        address: signer.address(),
                                        provider: validator_provider,
                                        delegators: delegator_infos,
                                    });
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Background: Failed to create staking transactions for validator #{validator_index}: {e:?}"
                                );
                            }
                        }
                        validator_index += 1;
                    }

                    1 => {
                        // Update consensus keys on a random registered validator
                        if registered_validators.is_empty() {
                            continue;
                        }
                        let idx = rng.gen_range(0..registered_validators.len());
                        let validator = &registered_validators[idx];

                        let seed = (validator.index * 10000 + i).to_le_bytes();
                        let mut new_seed = [0u8; 32];
                        new_seed[..8].copy_from_slice(&seed);

                        let new_bls_key = BLSKeyPair::generate(&mut StdRng::from_seed(new_seed));
                        let new_schnorr_key =
                            StateKeyPair::generate_from_seed_indexed(new_seed, validator.index);

                        let bls_sig: G1PointSol =
                            sign_address_bls(&new_bls_key, validator.address).into();
                        let schnorr_sig: StateSignatureSol =
                            sign_address_schnorr(&new_schnorr_key, validator.address).into();

                        match StakeTableV2::new(stake_table_addr, &validator.provider)
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
                                Ok(receipt) => println!(
                                    "Background: Updated keys for validator #{} (tx: {:?})",
                                    validator.index, receipt.transaction_hash
                                ),
                                Err(e) => println!(
                                    "Background: Failed to get receipt for validator #{}: {e:?}",
                                    validator.index
                                ),
                            },
                            Err(e) => println!(
                                "Background: Failed to update keys for validator #{}: {e:?}",
                                validator.index
                            ),
                        }
                    }

                    2 => {
                        // Undelegate from a random delegator of a random validator
                        if registered_validators.is_empty() {
                            continue;
                        }
                        let val_idx = rng.gen_range(0..registered_validators.len());
                        let validator = &registered_validators[val_idx];

                        if !validator.delegators.is_empty() {
                            let del_idx = rng.gen_range(0..validator.delegators.len());
                            let delegator = &validator.delegators[del_idx];

                            let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
                            let stake_table = StakeTableV2::new(stake_table_addr, &provider);

                            if let Ok(delegated_amount) = stake_table
                                .delegations(validator.address, delegator.address)
                                .call()
                                .await
                                && delegated_amount > U256::ZERO
                            {
                                let undelegate_amount = delegated_amount / U256::from(2);

                                match StakeTableV2::new(stake_table_addr, &delegator.provider)
                                    .undelegate(validator.address, undelegate_amount)
                                    .send()
                                    .await
                                {
                                    Ok(pending) => match pending.get_receipt().await {
                                        Ok(receipt) => println!(
                                            "Background: Undelegated {} ESP from validator #{} by delegator {:?} (tx: {:?})",
                                            undelegate_amount / U256::from(10).pow(U256::from(18)),
                                            validator.index,
                                            delegator.address,
                                            receipt.transaction_hash
                                        ),
                                        Err(e) => println!(
                                            "Background: Failed to get undelegate receipt for validator #{}: {e:?}",
                                            validator.index
                                        ),
                                    },
                                    Err(e) => println!(
                                        "Background: Failed to undelegate from validator #{}: {e:?}",
                                        validator.index
                                    ),
                                }
                            }
                        }
                    }

                    3 => {
                        // Deregister a random validator
                        if registered_validators.is_empty() {
                            continue;
                        }
                        let idx = rng.gen_range(0..registered_validators.len());
                        let validator = &registered_validators[idx];

                        match StakeTableV2::new(stake_table_addr, &validator.provider)
                            .deregisterValidator()
                            .send()
                            .await
                        {
                            Ok(pending) => match pending.get_receipt().await {
                                Ok(_) => {
                                    println!(
                                        "Background: Validator #{} deregistered",
                                        validator.index,
                                    );

                                    registered_validators.remove(idx);
                                }
                                Err(e) => println!(
                                    "Background: Failed to get deregister receipt for validator #{}: {e:?}",
                                    validator.index
                                ),
                            },
                            Err(e) => {
                                println!(
                                    "Background: Failed to deregister validator #{}: {e:?}",
                                    validator.index
                                );
                            }
                        }
                    }

                    _ => unreachable!(),
                }

                sleep(Duration::from_millis(500)).await;
            }
        });

        Self {
            task_handle: Some(task_handle),
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
