use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    path::PathBuf,
    time::Duration,
};

use alloy::{
    contract::Error as ContractError,
    network::{Ethereum, EthereumWallet, TransactionBuilder as _},
    primitives::{
        utils::{format_ether, parse_ether},
        Address, U256,
    },
    providers::{PendingTransactionBuilder, Provider, ProviderBuilder, WalletProvider},
    rpc::{
        client::RpcClient,
        types::{TransactionReceipt, TransactionRequest},
    },
    signers::local::PrivateKeySigner,
    transports::{http::Http, TransportError},
};
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use espresso_contract_deployer::{build_provider, build_signer, HttpProviderWithWallet};
use espresso_types::parse_duration;
use futures_util::future;
use hotshot_contract_adapter::{
    sol_types::{EspToken, StakeTableV2},
    stake_table::StakeTableContractVersion,
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use thiserror::Error;
use url::Url;

use crate::{
    info::fetch_token_address,
    parse::{parse_bls_priv_key, parse_state_priv_key, Commission, ParseCommissionError},
    receipt::ReceiptExt as _,
    signature::NodeSignatures,
    transaction::Transaction,
    tx_log::{execute_signed_tx_log, sign_all_transactions, TxInput, TxLog, TxPhase},
    Config, DEMO_VALIDATOR_START_INDEX,
};

#[derive(Debug, Error)]
pub enum CreateTransactionsError {
    #[error(
        "insufficient ESP balance: have {have} ESP, need {need} ESP to fund {delegators} \
         delegators"
    )]
    InsufficientEsp {
        have: String,
        need: String,
        delegators: usize,
    },
    #[error(
        "insufficient ETH balance: have {have} ETH, need {need} ETH (including gas buffer) to \
         fund {recipients} recipients"
    )]
    InsufficientEth {
        have: String,
        need: String,
        recipients: usize,
    },
    #[error("delegation amount {amount} ESP is below minimum of {min} ESP")]
    DelegationBelowMinimum { amount: String, min: String },
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Commission(#[from] ParseCommissionError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Args, Debug, Clone)]
pub struct Demo {
    #[command(subcommand)]
    pub command: DemoCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DemoCommands {
    /// Register validators and create delegators for demo
    Stake {
        /// The number of validators to register.
        #[clap(long, default_value_t = 5)]
        num_validators: u16,

        /// The number of delegators to create per validator.
        #[clap(long, env = "NUM_DELEGATORS_PER_VALIDATOR", value_parser = clap::value_parser!(u64).range(..=100000))]
        num_delegators_per_validator: Option<u64>,

        #[clap(long, value_enum, env = "DELEGATION_CONFIG", default_value_t = DelegationConfig::default())]
        delegation_config: DelegationConfig,

        /// Number of concurrent transaction submissions
        #[clap(long, default_value_t = crate::tx_log::DEFAULT_CONCURRENCY)]
        concurrency: usize,
    },
    /// Mass delegate to existing validators
    Delegate {
        /// Comma-separated validator addresses to delegate to
        #[clap(long, value_delimiter = ',')]
        validators: Vec<Address>,

        /// Starting index for delegator generation
        #[clap(long)]
        delegator_start_index: u64,

        /// Number of delegators to create
        #[clap(long)]
        num_delegators: u64,

        /// Minimum delegation amount (ESP)
        #[clap(long, value_parser = parse_ether)]
        min_amount: U256,

        /// Maximum delegation amount (ESP)
        #[clap(long, value_parser = parse_ether)]
        max_amount: U256,

        /// Path to transaction log file for recoverable execution
        #[clap(long, default_value_os_t = crate::default_tx_log_path())]
        log_path: PathBuf,

        /// Number of concurrent transaction submissions
        #[clap(long, default_value_t = crate::tx_log::DEFAULT_CONCURRENCY)]
        concurrency: usize,
    },
    /// Mass undelegate from validators
    Undelegate {
        /// Comma-separated validator addresses to undelegate from
        #[clap(long, value_delimiter = ',')]
        validators: Vec<Address>,

        /// Starting index for delegator generation
        #[clap(long)]
        delegator_start_index: u64,

        /// Number of delegators
        #[clap(long)]
        num_delegators: u64,

        /// Path to transaction log file for recoverable execution
        #[clap(long, default_value_os_t = crate::default_tx_log_path())]
        log_path: PathBuf,

        /// Number of concurrent transaction submissions
        #[clap(long, default_value_t = crate::tx_log::DEFAULT_CONCURRENCY)]
        concurrency: usize,
    },
    /// Deploy staking contracts for testing (requires --features testing)
    #[cfg(feature = "testing")]
    DeployContracts {
        /// Path to output .env file with contract addresses
        #[clap(long, default_value = ".env.contracts")]
        output: PathBuf,
    },
    /// Continuous delegation/undelegation activity
    Churn {
        /// Starting mnemonic index for validators
        #[clap(long, default_value_t = 20)]
        validator_start_index: u32,

        /// Number of validators to target
        #[clap(long)]
        num_validators: u16,

        /// Starting index for delegator generation
        #[clap(long)]
        delegator_start_index: u64,

        /// Number of delegators in the pool
        #[clap(long)]
        num_delegators: u64,

        /// Minimum delegation amount (ESP)
        #[clap(long, value_parser = parse_ether)]
        min_amount: U256,

        /// Maximum delegation amount (ESP)
        #[clap(long, value_parser = parse_ether)]
        max_amount: U256,

        /// Delay between operations
        #[clap(long, value_parser = parse_duration, default_value = "1s")]
        delay: Duration,

        /// Number of concurrent transaction submissions for initial funding
        #[clap(long, default_value_t = crate::tx_log::DEFAULT_CONCURRENCY)]
        concurrency: usize,
    },
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum DelegationConfig {
    EqualAmounts,
    #[default]
    VariableAmounts,
    MultipleDelegators,
    NoSelfDelegation,
}

// Manual implementation to match parsing of clap's ValueEnum
impl fmt::Display for DelegationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DelegationConfig::EqualAmounts => "equal-amounts",
            DelegationConfig::VariableAmounts => "variable-amounts",
            DelegationConfig::MultipleDelegators => "multiple-delegators",
            DelegationConfig::NoSelfDelegation => "no-self-delegation",
        };
        write!(f, "{s}")
    }
}

/// Validator registration info used by staking UI service tests.
///
/// Retrieved after calling `staking_cli::demo::create()` to get validator addresses.
/// The staking UI service tests use these addresses to verify that registration
/// events are correctly processed on the L1 stake table contract.
#[derive(Clone, Debug)]
pub struct RegistrationInfo {
    pub from: Address,
    pub commission: Commission,
    pub payload: NodeSignatures,
}

/// Delegation info used by staking UI service tests.
///
/// Retrieved after calling `staking_cli::demo::create()` to get delegator addresses.
/// The staking UI service tests use these addresses to verify that delegation
/// events are correctly processed on the L1 stake table contract.
#[derive(Clone, Debug)]
pub struct DelegationInfo {
    pub from: Address,
    pub validator: Address,
    pub amount: U256,
}

#[derive(Clone, Debug)]
enum StakeTableTx {
    SendEth {
        to: Address,
        amount: U256,
    },
    SendEsp {
        to: Address,
        amount: U256,
    },
    RegisterValidator {
        from: Address,
        commission: Commission,
        payload: Box<NodeSignatures>,
    },
    Approve {
        from: Address,
        amount: U256,
    },
    Delegate {
        from: Address,
        validator: Address,
        amount: U256,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupPhase {
    Funding,
    Approval,
    Registration,
    Delegation,
}

impl SetupPhase {
    fn next(self) -> Option<Self> {
        match self {
            Self::Funding => Some(Self::Approval),
            Self::Approval => Some(Self::Registration),
            Self::Registration => Some(Self::Delegation),
            Self::Delegation => None,
        }
    }
}

struct ValidatorConfig {
    signer: PrivateKeySigner,
    commission: Commission,
    bls_key_pair: BLSKeyPair,
    state_key_pair: StateKeyPair,
    index: usize,
}

struct DelegatorConfig {
    validator: Address,
    signer: PrivateKeySigner,
    delegate_amount: U256,
}

#[derive(Clone, Debug)]
struct TransactionQueues {
    funding: VecDeque<StakeTableTx>,
    approvals: VecDeque<StakeTableTx>,
    registration: VecDeque<StakeTableTx>,
    delegations: VecDeque<StakeTableTx>,
    current_phase: SetupPhase,
}

impl TransactionQueues {
    fn current_group_mut(&mut self) -> &mut VecDeque<StakeTableTx> {
        match self.current_phase {
            SetupPhase::Funding => &mut self.funding,
            SetupPhase::Approval => &mut self.approvals,
            SetupPhase::Registration => &mut self.registration,
            SetupPhase::Delegation => &mut self.delegations,
        }
    }

    fn pop_next(&mut self) -> Option<StakeTableTx> {
        loop {
            if let Some(tx) = self.current_group_mut().pop_front() {
                return Some(tx);
            }
            self.current_phase = self.current_phase.next()?;
        }
    }
}

#[derive(Clone, Debug)]
struct TransactionProcessor<P> {
    providers: HashMap<Address, P>,
    funder: P,
    stake_table: Address,
    token: Address,
    version: StakeTableContractVersion,
}

impl<P: Provider + Clone> TransactionProcessor<P> {
    fn provider(&self, address: Address) -> Result<&P> {
        self.providers
            .get(&address)
            .ok_or_else(|| anyhow::anyhow!("provider not found for {address}"))
    }

    async fn send_next(&self, tx: StakeTableTx) -> Result<PendingTransactionBuilder<Ethereum>> {
        match tx {
            StakeTableTx::SendEth { to, amount } => {
                let tx = TransactionRequest::default().with_to(to).with_value(amount);
                Ok(self.funder.send_transaction(tx).await?)
            },
            StakeTableTx::SendEsp { to, amount } => {
                Transaction::Transfer {
                    token: self.token,
                    to,
                    amount,
                }
                .send(&self.funder)
                .await
            },
            StakeTableTx::RegisterValidator {
                from,
                commission,
                payload,
            } => {
                let metadata_uri = "https://example.com/metadata".parse()?;
                Transaction::RegisterValidator {
                    stake_table: self.stake_table,
                    commission,
                    metadata_uri,
                    payload: *payload,
                    version: self.version,
                }
                .send(self.provider(from)?)
                .await
            },
            StakeTableTx::Approve { from, amount } => {
                Transaction::Approve {
                    token: self.token,
                    spender: self.stake_table,
                    amount,
                }
                .send(self.provider(from)?)
                .await
            },
            StakeTableTx::Delegate {
                from,
                validator,
                amount,
            } => {
                Transaction::Delegate {
                    stake_table: self.stake_table,
                    validator,
                    amount,
                }
                .send(self.provider(from)?)
                .await
            },
        }
    }

    async fn process_group(
        &self,
        txs: &mut VecDeque<StakeTableTx>,
    ) -> Result<Vec<TransactionReceipt>> {
        let mut pending = vec![];
        while let Some(tx) = txs.pop_front() {
            pending.push(self.send_next(tx).await?);
        }
        future::try_join_all(
            pending
                .into_iter()
                .map(|p| async move { p.assert_success().await }),
        )
        .await
    }
}

#[derive(Clone, Debug)]
pub struct StakingTransactions<P> {
    processor: TransactionProcessor<P>,
    queues: TransactionQueues,
}

impl<P: Provider + Clone> StakingTransactions<P> {
    /// Sends and awaits all transactions with high concurrency.
    ///
    /// This is the preferred way to make the changes to the stake table
    /// contract as quickly as possible, while still allowing alloy's implicit
    /// estimateGas calls to succeed.
    ///
    /// Ensures that dependent transactions are finalized before
    /// continuing.
    ///
    /// The synchronization points are after
    ///
    /// 1. Ether + token funding
    /// 2. Approvals
    /// 3. Registrations
    /// 4. Delegations
    ///
    /// For each them at least one L1 block will be required.
    pub async fn apply_all(&mut self) -> Result<Vec<TransactionReceipt>> {
        let mut receipts = Vec::new();

        for queue in [
            &mut self.queues.funding,
            &mut self.queues.approvals,
            &mut self.queues.registration,
            &mut self.queues.delegations,
        ] {
            receipts.extend(self.processor.process_group(queue).await?);
        }

        tracing::info!("completed all staking transactions");

        Ok(receipts)
    }

    /// Sends and awaits receipts on all funding and approval transactions
    ///
    /// If the caller wants more control but quickly get to a point where actual
    /// changes are made to the stake table it is useful to call this function
    /// first.
    ///
    /// This processes funding and approvals with a synchronization point between them.
    pub async fn apply_prerequisites(&mut self) -> Result<Vec<TransactionReceipt>> {
        if !matches!(self.queues.current_phase, SetupPhase::Funding) {
            return Err(anyhow::anyhow!("apply_prerequisites must be called first"));
        }

        let mut receipts = Vec::new();

        for queue in [&mut self.queues.funding, &mut self.queues.approvals] {
            receipts.extend(self.processor.process_group(queue).await?);
        }

        self.queues.current_phase = SetupPhase::Registration;

        Ok(receipts)
    }

    /// Sends and awaits one transaction
    ///
    /// The caller can use this function to rate limit changes to the L1 stake
    /// table contract during setup.
    pub async fn apply_one(&mut self) -> Result<Option<TransactionReceipt>> {
        let Some(tx) = self.queues.pop_next() else {
            return Ok(None);
        };
        let pending = self.processor.send_next(tx).await?;
        Ok(Some(pending.assert_success().await?))
    }

    /// Sends and awaits all transactions with backpressure using the tx_log pattern.
    ///
    /// This method pre-signs all transactions and executes them with bounded concurrency,
    /// providing backpressure when the node is overloaded. Registration transactions are
    /// handled sequentially since they are typically few in number and have complex calldata.
    ///
    /// The phases are executed in order with synchronization between:
    /// 1. Funding (ETH + ESP sends) and Approvals
    /// 2. Registrations (sequential)
    /// 3. Delegations
    ///
    /// Delegations must happen after registrations because you cannot delegate to a validator
    /// that doesn't exist yet.
    pub async fn apply_with_backpressure(&mut self, concurrency: usize) -> Result<()>
    where
        P: WalletProvider<Wallet = EthereumWallet> + Clone + 'static,
    {
        tracing::info!(
            "applying staking transactions with backpressure (concurrency={})",
            concurrency
        );

        let token = self.processor.token;
        let stake_table = self.processor.stake_table;

        // Build wallets map from providers
        let wallets: HashMap<Address, EthereumWallet> = self
            .processor
            .providers
            .iter()
            .map(|(addr, p)| (*addr, p.wallet().clone()))
            .collect();

        // Get funder wallet
        let funder_address = self.processor.funder.default_signer_address();
        let funder_wallet = self.processor.funder.wallet().clone();

        // Build complete wallets map including funder
        let mut all_wallets = wallets.clone();
        all_wallets.insert(funder_address, funder_wallet);

        // Phase 1: Build TxInput entries for funding and approvals only (no delegations yet)
        let mut funding_approval_inputs: Vec<TxInput> = Vec::new();

        // Funding phase: ETH sends then ESP sends (from funder)
        for tx in &self.queues.funding {
            match tx {
                StakeTableTx::SendEth { to, amount } => {
                    funding_approval_inputs.push(TxInput {
                        phase: TxPhase::FundEth,
                        from: funder_address,
                        to: *to,
                        amount: *amount,
                        delegator_index: None,
                    });
                },
                StakeTableTx::SendEsp { to, amount } => {
                    funding_approval_inputs.push(TxInput {
                        phase: TxPhase::FundEsp,
                        from: funder_address,
                        to: *to,
                        amount: *amount,
                        delegator_index: None,
                    });
                },
                _ => {},
            }
        }

        // Approval phase
        for tx in &self.queues.approvals {
            if let StakeTableTx::Approve { from, amount } = tx {
                funding_approval_inputs.push(TxInput {
                    phase: TxPhase::Approve,
                    from: *from,
                    to: stake_table,
                    amount: *amount,
                    delegator_index: None,
                });
            }
        }

        // Execute funding and approvals
        if !funding_approval_inputs.is_empty() {
            tracing::info!(
                "signing {} funding/approval transactions...",
                funding_approval_inputs.len()
            );
            let signed_txs = sign_all_transactions(
                &self.processor.funder,
                &all_wallets,
                funding_approval_inputs,
                concurrency,
                |input| {
                    match input.phase {
                        TxPhase::FundEth => TransactionRequest::default()
                            .with_to(input.to)
                            .with_value(input.amount),
                        TxPhase::FundEsp => {
                            let call = EspToken::transferCall {
                                to: input.to,
                                value: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(token)
                                .with_call(&call)
                        },
                        TxPhase::Approve => {
                            let call = EspToken::approveCall {
                                spender: stake_table,
                                value: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(token)
                                .with_call(&call)
                        },
                        // Delegate and Undelegate are not included in funding/approval phase
                        TxPhase::Delegate | TxPhase::Undelegate => unreachable!(),
                    }
                },
            )
            .await?;

            let log = TxLog::new(signed_txs);
            execute_signed_tx_log(self.processor.funder.clone(), &log, concurrency, false).await?;
        }

        // Phase 2: Handle registrations sequentially (typically few transactions, complex calldata)
        if !self.queues.registration.is_empty() {
            tracing::info!(
                "processing {} registrations sequentially",
                self.queues.registration.len()
            );
            for tx in std::mem::take(&mut self.queues.registration) {
                let pending = self.processor.send_next(tx).await?;
                pending.assert_success().await?;
            }
        }

        // Phase 3: Build TxInput entries for delegations (after registrations complete)
        let mut delegation_inputs: Vec<TxInput> = Vec::new();

        for tx in &self.queues.delegations {
            if let StakeTableTx::Delegate {
                from,
                validator,
                amount,
            } = tx
            {
                delegation_inputs.push(TxInput {
                    phase: TxPhase::Delegate,
                    from: *from,
                    to: *validator,
                    amount: *amount,
                    delegator_index: None,
                });
            }
        }

        // Execute delegations
        if !delegation_inputs.is_empty() {
            tracing::info!(
                "signing {} delegation transactions...",
                delegation_inputs.len()
            );
            let signed_txs = sign_all_transactions(
                &self.processor.funder,
                &all_wallets,
                delegation_inputs,
                concurrency,
                |input| {
                    match input.phase {
                        TxPhase::Delegate => {
                            let call = StakeTableV2::delegateCall {
                                validator: input.to,
                                amount: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(stake_table)
                                .with_call(&call)
                        },
                        // Only Delegate phase is expected here; Undelegate is never used in
                        // apply_with_backpressure (it's only for the undelegate_for_demo flow)
                        TxPhase::FundEth
                        | TxPhase::FundEsp
                        | TxPhase::Approve
                        | TxPhase::Undelegate => {
                            unreachable!()
                        },
                    }
                },
            )
            .await?;

            let log = TxLog::new(signed_txs);
            execute_signed_tx_log(self.processor.funder.clone(), &log, concurrency, false).await?;
        }

        // Clear processed queues
        self.queues.funding.clear();
        self.queues.approvals.clear();
        self.queues.delegations.clear();
        self.queues.current_phase = SetupPhase::Delegation;

        tracing::info!("completed all staking transactions with backpressure");
        Ok(())
    }

    /// Returns pending validator registrations for staking UI service tests.
    ///
    /// Retrieves validator addresses that were set up by `staking_cli::demo::create()`.
    /// Tests use these addresses to verify registration event processing.
    pub fn registrations(&self) -> Vec<RegistrationInfo> {
        self.queues
            .registration
            .iter()
            .filter_map(|tx| {
                if let StakeTableTx::RegisterValidator {
                    from,
                    commission,
                    payload,
                } = tx
                {
                    Some(RegistrationInfo {
                        from: *from,
                        commission: *commission,
                        payload: *payload.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns pending delegations for staking UI service tests.
    ///
    /// Used to retrieve delegator and validator addresses after calling `staking_cli::demo::create()`.
    /// Tests use this data to verify delegation event processing in the staking UI service.
    pub fn delegations(&self) -> Vec<DelegationInfo> {
        self.queues
            .delegations
            .iter()
            .filter_map(|tx| {
                if let StakeTableTx::Delegate {
                    from,
                    validator,
                    amount,
                } = tx
                {
                    Some(DelegationInfo {
                        from: *from,
                        validator: *validator,
                        amount: *amount,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the provider for a given address.
    ///
    /// This is useful when you need to get the provider for a delegator or validator
    /// that was created during `StakingTransactions::create()`.
    ///
    /// Currently this is used by staking UI service to get the provider
    /// so that we can also undelegate stake because we need the signer that
    /// signed the delegate transaction
    pub fn provider(&self, address: Address) -> Option<&P> {
        self.processor.providers.get(&address)
    }
}

impl StakingTransactions<HttpProviderWithWallet> {
    /// Create staking transactions for test setup
    ///
    /// Prepares all transactions needed to setup the stake table with validators and delegations.
    /// The transactions can be applied with different levels of concurrency using the methods on
    /// the returned instance.
    ///
    /// Amounts used for funding, delegations, number of delegators are chosen somewhat arbitrarily.
    ///
    /// Assumptions:
    ///
    /// - Full control of Validators Ethereum wallets and the Ethereum node. Transactions are
    ///   constructed in a way that they should always apply, if some (but not all) transactions
    ///   fail to apply the easiest fix is probably to re-deploy the Ethereum network. Recovery,
    ///   replacing of transactions is not implemented.
    ///
    /// - Nobody else is using the Ethereum accounts for anything else between calling this function
    ///   and applying the returned transactions.
    ///
    /// Requirements:
    ///
    /// - token_holder: Requires Eth to fund validators and delegators, ESP tokens to fund delegators.
    ///
    /// Errors:
    ///
    /// - If Eth or ESP balances of the token_holder are insufficient.
    /// - If any RPC request to the Ethereum node or contract calls fail.
    pub async fn create(
        rpc_url: Url,
        token_holder: &(impl Provider + WalletProvider<Wallet = EthereumWallet>),
        stake_table: Address,
        validators: Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)>,
        num_delegators_per_validator: Option<u64>,
        config: DelegationConfig,
    ) -> Result<Self, CreateTransactionsError> {
        tracing::info!(%stake_table, "staking to stake table contract for demo");

        let token = fetch_token_address(rpc_url.clone(), stake_table).await?;

        // Create shared HTTP client to reuse connection pool across all providers. Avoids creating
        // too many connections to our geth node.
        let shared_client = RpcClient::new(Http::new(rpc_url), /*is_local*/ true);

        let token_holder_provider = ProviderBuilder::new()
            .wallet(token_holder.wallet().clone())
            .connect_client(shared_client.clone());

        tracing::info!("ESP token address: {token}");
        let token_holder_addr = token_holder.default_signer_address();
        let token_balance = EspToken::new(token, &token_holder_provider)
            .balanceOf(token_holder_addr)
            .call()
            .await?;
        tracing::info!(
            "token distributor account {} balance: {} ESP",
            token_holder_addr,
            format_ether(token_balance)
        );

        let fund_amount_esp = parse_ether("1000").unwrap();
        let fund_amount_eth = parse_ether("10").unwrap();

        let seed = [42u8; 32];
        let mut rng = ChaCha20Rng::from_seed(seed);

        let mut validator_info = vec![];
        for (val_index, (signer, bls_key_pair, state_key_pair)) in
            validators.into_iter().enumerate()
        {
            let commission = Commission::try_from(100u64 + 10u64 * val_index as u64)?;

            validator_info.push(ValidatorConfig {
                signer,
                commission,
                bls_key_pair,
                state_key_pair,
                index: val_index,
            });
        }

        let mut delegator_info = vec![];

        for validator in &validator_info {
            let delegate_amount = match config {
                DelegationConfig::EqualAmounts => Some(parse_ether("100").unwrap()),
                DelegationConfig::MultipleDelegators | DelegationConfig::VariableAmounts => {
                    Some(parse_ether("100").unwrap() * U256::from(validator.index % 5 + 1))
                },
                DelegationConfig::NoSelfDelegation => None,
            };

            if let Some(amount) = delegate_amount {
                delegator_info.push(DelegatorConfig {
                    validator: validator.signer.address(),
                    signer: validator.signer.clone(),
                    delegate_amount: amount,
                });
            }
        }

        if matches!(
            config,
            DelegationConfig::MultipleDelegators | DelegationConfig::NoSelfDelegation
        ) {
            for validator in &validator_info {
                let delegators_per_validator = num_delegators_per_validator
                    .map(|n| n as usize)
                    .unwrap_or_else(|| rng.gen_range(2..=5));
                for _ in 0..delegators_per_validator {
                    let random_amount: u64 = rng.gen_range(100..=500);
                    delegator_info.push(DelegatorConfig {
                        validator: validator.signer.address(),
                        signer: PrivateKeySigner::random(),
                        delegate_amount: parse_ether(&random_amount.to_string()).unwrap(),
                    });
                }
            }
        }

        let st = StakeTableV2::new(stake_table, &token_holder_provider);
        let version: StakeTableContractVersion = st.getVersion().call().await?.try_into()?;
        if let StakeTableContractVersion::V2 = version {
            let min_delegate_amount = st.minDelegateAmount().call().await?;
            for delegator in &delegator_info {
                if delegator.delegate_amount < min_delegate_amount {
                    return Err(CreateTransactionsError::DelegationBelowMinimum {
                        amount: format_ether(delegator.delegate_amount),
                        min: format_ether(min_delegate_amount),
                    });
                }
            }
        }

        let mut funding = VecDeque::new();

        let eth_recipients: HashSet<Address> = validator_info
            .iter()
            .map(|v| v.signer.address())
            .chain(delegator_info.iter().map(|d| d.signer.address()))
            .collect();

        for &address in &eth_recipients {
            funding.push_back(StakeTableTx::SendEth {
                to: address,
                amount: fund_amount_eth,
            });
        }

        for delegator in &delegator_info {
            funding.push_back(StakeTableTx::SendEsp {
                to: delegator.signer.address(),
                amount: fund_amount_esp,
            });
        }

        // Only create one provider per address to avoid nonce errors.
        let mut providers: HashMap<Address, _> = HashMap::new();

        let mut registration = VecDeque::new();

        for validator in &validator_info {
            let address = validator.signer.address();
            providers.entry(address).or_insert_with(|| {
                ProviderBuilder::new()
                    .wallet(EthereumWallet::from(validator.signer.clone()))
                    .connect_client(shared_client.clone())
            });

            let payload =
                NodeSignatures::create(address, &validator.bls_key_pair, &validator.state_key_pair);
            registration.push_back(StakeTableTx::RegisterValidator {
                from: address,
                commission: validator.commission,
                payload: Box::new(payload),
            });
        }

        let mut approvals = VecDeque::new();
        let mut delegations = VecDeque::new();

        for delegator in &delegator_info {
            let address = delegator.signer.address();
            providers.entry(address).or_insert_with(|| {
                ProviderBuilder::new()
                    .wallet(EthereumWallet::from(delegator.signer.clone()))
                    .connect_client(shared_client.clone())
            });

            approvals.push_back(StakeTableTx::Approve {
                from: address,
                amount: delegator.delegate_amount,
            });

            delegations.push_back(StakeTableTx::Delegate {
                from: address,
                validator: delegator.validator,
                amount: delegator.delegate_amount,
            });
        }

        let esp_required = fund_amount_esp * U256::from(delegator_info.len());
        let eth_required = fund_amount_eth * U256::from(eth_recipients.len()) * U256::from(2);

        tracing::info!(
            "Balance check: have {} ESP, need {} ESP for {} delegators",
            format_ether(token_balance),
            format_ether(esp_required),
            delegator_info.len()
        );

        if token_balance < esp_required {
            return Err(CreateTransactionsError::InsufficientEsp {
                have: format_ether(token_balance),
                need: format_ether(esp_required),
                delegators: delegator_info.len(),
            });
        }

        let eth_balance = token_holder_provider.get_balance(token_holder_addr).await?;

        tracing::info!(
            "Balance check: have {} ETH, need {} ETH for {} recipients",
            format_ether(eth_balance),
            format_ether(eth_required),
            eth_recipients.len()
        );

        if eth_balance < eth_required {
            return Err(CreateTransactionsError::InsufficientEth {
                have: format_ether(eth_balance),
                need: format_ether(eth_required),
                recipients: eth_recipients.len(),
            });
        }

        Ok(StakingTransactions {
            processor: TransactionProcessor {
                providers,
                funder: token_holder_provider,
                stake_table,
                token,
                version,
            },
            queues: TransactionQueues {
                funding,
                approvals,
                registration,
                delegations,
                current_phase: SetupPhase::Funding,
            },
        })
    }
}

const DELEGATOR_SEED: u64 = 42;

pub fn generate_delegator_signer(index: u64) -> PrivateKeySigner {
    let seed = DELEGATOR_SEED.wrapping_add(index);
    let mut seed_bytes = [0u8; 32];
    seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());
    let mut rng = ChaCha20Rng::from_seed(seed_bytes);
    PrivateKeySigner::random_with(&mut rng)
}

/// Register validators, and delegate to themselves for demo purposes.
///
/// The environment variables used only for this function but not for the normal staking CLI are
/// loaded directly from the environment.
///
/// Account indexes 20+ of the dev mnemonic are used for the validator accounts.
pub async fn stake_for_demo(
    config: &Config,
    num_validators: u16,
    num_delegators_per_validator: Option<u64>,
    delegation_config: DelegationConfig,
    concurrency: usize,
) -> Result<()> {
    tracing::info!("staking to stake table contract for demo");

    // let grant_recipient = mk_signer(config.signer.account_index.unwrap())?;
    let grant_recipient = build_provider(
        config.signer.mnemonic.clone().unwrap(),
        config.signer.account_index.unwrap(),
        config.rpc_url.clone(),
        /* polling_interval */ None,
    );

    tracing::info!(
        "grant recipient account for token funding: {}",
        grant_recipient.default_signer_address()
    );

    let token_address =
        fetch_token_address(config.rpc_url.clone(), config.stake_table_address).await?;
    tracing::info!("ESP token address: {}", token_address);
    let stake_table_address = config.stake_table_address;
    tracing::info!("stake table address: {}", stake_table_address);

    let mut validator_keys = vec![];
    for val_index in 0..num_validators {
        let signer = build_signer(
            config.signer.mnemonic.clone().unwrap(),
            DEMO_VALIDATOR_START_INDEX + val_index as u32,
        );

        let consensus_private_key = parse_bls_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_{val_index}"
        ))?)?
        .into();
        let state_private_key = parse_state_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_{val_index}"
        ))?)?;
        validator_keys.push((
            signer,
            consensus_private_key,
            StateKeyPair::from_sign_key(state_private_key),
        ));
    }

    StakingTransactions::create(
        config.rpc_url.clone(),
        &grant_recipient,
        config.stake_table_address,
        validator_keys,
        num_delegators_per_validator,
        delegation_config,
    )
    .await?
    .apply_with_backpressure(concurrency)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn delegate_for_demo(
    config: &Config,
    validators: Vec<Address>,
    delegator_start_index: u64,
    num_delegators: u64,
    min_amount: U256,
    max_amount: U256,
    log_path: PathBuf,
    concurrency: usize,
) -> Result<()> {
    tracing::info!("mass delegating to {} validators", validators.len());

    let grant_recipient = build_provider(
        config.signer.mnemonic.clone().unwrap(),
        config.signer.account_index.unwrap(),
        config.rpc_url.clone(),
        None,
    );

    let token_address =
        fetch_token_address(config.rpc_url.clone(), config.stake_table_address).await?;

    let fund_amount_esp = max_amount + parse_ether("100").unwrap();
    let fund_amount_eth = parse_ether("10").unwrap();

    let shared_client = RpcClient::new(Http::new(config.rpc_url.clone()), true);
    let funder_provider = ProviderBuilder::new()
        .wallet(grant_recipient.wallet().clone())
        .connect_client(shared_client.clone());

    let seed_offset = DELEGATOR_SEED.wrapping_add(delegator_start_index);
    let mut rng = ChaCha20Rng::seed_from_u64(seed_offset);

    struct DelegatorInfo {
        signer: PrivateKeySigner,
        validator: Address,
        amount: U256,
    }

    let mut delegators = Vec::with_capacity(num_delegators as usize);
    for i in 0..num_delegators {
        let delegator_index = delegator_start_index + i;
        let delegator_signer = generate_delegator_signer(delegator_index);
        let validator = validators[(i as usize) % validators.len()];

        let delegation_amount = if min_amount == max_amount {
            min_amount
        } else {
            let range = max_amount - min_amount;
            let random_offset = U256::from(rng.gen_range(0..=u128::MAX)) % range;
            min_amount + random_offset
        };

        tracing::debug!(
            "delegator {} (index {}) -> validator {}: {} ESP",
            delegator_signer.address(),
            delegator_index,
            validator,
            format_ether(delegation_amount)
        );

        delegators.push(DelegatorInfo {
            signer: delegator_signer,
            validator,
            amount: delegation_amount,
        });
    }

    tracing::info!(
        "using tx_log for recoverable execution (concurrency={})",
        concurrency
    );

    let funder_address = grant_recipient.default_signer_address();
    let stake_table_address = config.stake_table_address;

    let log = match TxLog::load(&log_path)? {
        Some(existing) => {
            tracing::info!(
                "resuming from tx log at {} ({} txs)",
                log_path.display(),
                existing.transactions.len()
            );
            existing
        },
        None => {
            // Check funder has sufficient balance before creating any transactions
            let esp_required = fund_amount_esp * U256::from(delegators.len());
            // ETH required: fund each delegator + gas buffer (2x the funding amount)
            let eth_required = fund_amount_eth * U256::from(delegators.len()) * U256::from(2);

            let token_contract = EspToken::new(token_address, &funder_provider);
            let esp_balance = token_contract.balanceOf(funder_address).call().await?;

            tracing::info!(
                "Balance check: have {} ESP, need {} ESP for {} delegators",
                format_ether(esp_balance),
                format_ether(esp_required),
                delegators.len()
            );

            if esp_balance < esp_required {
                return Err(CreateTransactionsError::InsufficientEsp {
                    have: format_ether(esp_balance),
                    need: format_ether(esp_required),
                    delegators: delegators.len(),
                }
                .into());
            }

            let eth_balance = funder_provider.get_balance(funder_address).await?;

            tracing::info!(
                "Balance check: have {} ETH, need {} ETH for {} delegators",
                format_ether(eth_balance),
                format_ether(eth_required),
                delegators.len()
            );

            if eth_balance < eth_required {
                return Err(CreateTransactionsError::InsufficientEth {
                    have: format_ether(eth_balance),
                    need: format_ether(eth_required),
                    recipients: delegators.len(),
                }
                .into());
            }

            let total_txs = delegators.len() * 4;
            tracing::info!(
                "creating tx log at {} ({} transactions)",
                log_path.display(),
                total_txs
            );

            let mut tx_inputs = Vec::with_capacity(total_txs);

            // Group by phase to ensure contiguous nonces per sender within each phase.
            // FundEth and FundEsp are from funder, so they need sequential nonces.
            // Approve and Delegate are from each delegator (1 tx each per phase).
            for d in delegators.iter() {
                tx_inputs.push(TxInput {
                    phase: TxPhase::FundEth,
                    from: funder_address,
                    to: d.signer.address(),
                    amount: fund_amount_eth,
                    delegator_index: None,
                });
            }
            for d in delegators.iter() {
                tx_inputs.push(TxInput {
                    phase: TxPhase::FundEsp,
                    from: funder_address,
                    to: d.signer.address(),
                    amount: fund_amount_esp,
                    delegator_index: None,
                });
            }
            for (i, d) in delegators.iter().enumerate() {
                let delegator_index = delegator_start_index + i as u64;
                tx_inputs.push(TxInput {
                    phase: TxPhase::Approve,
                    from: d.signer.address(),
                    to: stake_table_address,
                    amount: d.amount,
                    delegator_index: Some(delegator_index),
                });
            }
            for (i, d) in delegators.iter().enumerate() {
                let delegator_index = delegator_start_index + i as u64;
                tx_inputs.push(TxInput {
                    phase: TxPhase::Delegate,
                    from: d.signer.address(),
                    to: d.validator,
                    amount: d.amount,
                    delegator_index: Some(delegator_index),
                });
            }

            let mut wallets: HashMap<Address, EthereumWallet> = HashMap::new();
            wallets.insert(funder_address, grant_recipient.wallet().clone());
            for d in &delegators {
                wallets.insert(d.signer.address(), EthereumWallet::from(d.signer.clone()));
            }

            tracing::info!("signing {} transactions...", tx_inputs.len());
            let signed_txs = sign_all_transactions(
                &funder_provider,
                &wallets,
                tx_inputs,
                concurrency,
                |input| {
                    use alloy::{network::TransactionBuilder as _, rpc::types::TransactionRequest};
                    use hotshot_contract_adapter::sol_types::{EspToken, StakeTableV2};

                    match input.phase {
                        TxPhase::FundEth => TransactionRequest::default()
                            .with_to(input.to)
                            .with_value(input.amount),
                        TxPhase::FundEsp => {
                            let call = EspToken::transferCall {
                                to: input.to,
                                value: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(token_address)
                                .with_call(&call)
                        },
                        TxPhase::Approve => {
                            let call = EspToken::approveCall {
                                spender: stake_table_address,
                                value: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(token_address)
                                .with_call(&call)
                        },
                        TxPhase::Delegate => {
                            let call = StakeTableV2::delegateCall {
                                validator: input.to,
                                amount: input.amount,
                            };
                            TransactionRequest::default()
                                .with_to(stake_table_address)
                                .with_call(&call)
                        },
                        TxPhase::Undelegate => unreachable!(),
                    }
                },
            )
            .await?;

            let log = TxLog::new(signed_txs);
            log.save(&log_path)?;
            log
        },
    };

    execute_signed_tx_log(funder_provider, &log, concurrency, false).await?;

    log.archive(&log_path)?;
    tracing::info!("completed mass delegation");
    Ok(())
}

pub async fn undelegate_for_demo(
    config: &Config,
    validators: Vec<Address>,
    delegator_start_index: u64,
    num_delegators: u64,
    log_path: PathBuf,
    concurrency: usize,
) -> Result<()> {
    tracing::info!("mass undelegating from {} validators", validators.len());

    let shared_client = RpcClient::new(Http::new(config.rpc_url.clone()), true);
    let query_provider = ProviderBuilder::new().connect_client(shared_client.clone());

    struct UndelegationInfo {
        signer: PrivateKeySigner,
        validator: Address,
        amount: U256,
    }

    let log = match TxLog::load(&log_path)? {
        Some(existing) => {
            tracing::info!(
                "resuming from tx log at {} ({} txs)",
                log_path.display(),
                existing.transactions.len()
            );
            existing
        },
        None => {
            let mut queries = Vec::new();
            for i in 0..num_delegators {
                let delegator_index = delegator_start_index + i;
                let delegator_signer = generate_delegator_signer(delegator_index);
                for validator in &validators {
                    queries.push((delegator_signer.clone(), *validator));
                }
            }

            tracing::info!(
                "querying {} delegation amounts (concurrency={})",
                queries.len(),
                concurrency
            );

            let stake_table_addr = config.stake_table_address;
            let results =
                crate::concurrent::map_concurrent("querying delegations", queries, concurrency, {
                    let client = shared_client.clone();
                    move |(signer, validator): (PrivateKeySigner, Address)| {
                        let client = client.clone();
                        async move {
                            let provider = ProviderBuilder::new().connect_client(client);
                            let stake_table = StakeTableV2::new(stake_table_addr, &provider);
                            let amount = stake_table
                                .delegations(validator, signer.address())
                                .call()
                                .await?;
                            Ok((signer, validator, amount))
                        }
                    }
                })
                .await?;

            let mut undelegations = Vec::new();
            for (signer, validator, amount) in results {
                if amount.is_zero() {
                    continue;
                }
                tracing::debug!(
                    "undelegating delegator {} from validator {}: {} ESP",
                    signer.address(),
                    validator,
                    format_ether(amount)
                );
                undelegations.push(UndelegationInfo {
                    signer,
                    validator,
                    amount,
                });
            }

            if undelegations.is_empty() {
                tracing::info!("no delegations to undelegate");
                return Ok(());
            }

            tracing::info!("found {} delegations to undelegate", undelegations.len());

            let stake_table_address = config.stake_table_address;

            let tx_inputs: Vec<_> = undelegations
                .iter()
                .enumerate()
                .map(|(i, u)| TxInput {
                    phase: TxPhase::Undelegate,
                    from: u.signer.address(),
                    to: u.validator,
                    amount: u.amount,
                    delegator_index: Some(delegator_start_index + i as u64),
                })
                .collect();

            let wallets: HashMap<Address, EthereumWallet> = undelegations
                .iter()
                .map(|u| (u.signer.address(), EthereumWallet::from(u.signer.clone())))
                .collect();

            tracing::info!("signing {} transactions...", tx_inputs.len());
            let signed_txs =
                sign_all_transactions(&query_provider, &wallets, tx_inputs, concurrency, |input| {
                    use alloy::{network::TransactionBuilder as _, rpc::types::TransactionRequest};
                    use hotshot_contract_adapter::sol_types::StakeTableV2;

                    let call = StakeTableV2::undelegateCall {
                        validator: input.to,
                        amount: input.amount,
                    };
                    TransactionRequest::default()
                        .with_to(stake_table_address)
                        .with_call(&call)
                })
                .await?;

            let log = TxLog::new(signed_txs);
            log.save(&log_path)?;
            tracing::info!(
                "created tx log at {} ({} transactions)",
                log_path.display(),
                log.transactions.len()
            );
            log
        },
    };

    tracing::info!(
        "using tx_log for recoverable execution (concurrency={})",
        concurrency
    );

    execute_signed_tx_log(query_provider, &log, concurrency, false).await?;

    log.archive(&log_path)?;
    tracing::info!("completed mass undelegation");
    Ok(())
}

pub struct ChurnParams {
    pub validator_start_index: u32,
    pub num_validators: u16,
    pub delegator_start_index: u64,
    pub num_delegators: u64,
    pub min_amount: U256,
    pub max_amount: U256,
    pub delay: Duration,
    pub concurrency: usize,
}

pub async fn churn_for_demo(config: &Config, params: ChurnParams) -> Result<()> {
    let ChurnParams {
        validator_start_index,
        num_validators,
        delegator_start_index,
        num_delegators,
        min_amount,
        max_amount,
        delay,
        concurrency,
    } = params;
    tracing::info!(
        "starting churn with {} validators and {} delegators (concurrency={})",
        num_validators,
        num_delegators,
        concurrency
    );

    let mnemonic = config.signer.mnemonic.clone().unwrap();
    let mut validator_addresses = Vec::new();
    for i in 0..num_validators {
        let signer = build_signer(mnemonic.clone(), validator_start_index + i as u32);
        validator_addresses.push(signer.address());
    }

    let grant_recipient = build_provider(
        mnemonic.clone(),
        config.signer.account_index.unwrap(),
        config.rpc_url.clone(),
        None,
    );

    let token_address =
        fetch_token_address(config.rpc_url.clone(), config.stake_table_address).await?;
    let fund_amount_esp = max_amount + parse_ether("100").unwrap();
    let fund_amount_eth = parse_ether("10").unwrap();

    let shared_client = RpcClient::new(Http::new(config.rpc_url.clone()), true);
    let funder_provider = ProviderBuilder::new()
        .wallet(grant_recipient.wallet().clone())
        .connect_client(shared_client.clone());

    // Build delegator info
    let delegators: Vec<_> = (0..num_delegators)
        .map(|i| {
            let delegator_index = delegator_start_index + i;
            generate_delegator_signer(delegator_index)
        })
        .collect();

    // Check funder has sufficient balance before creating any transactions
    let funder_address = grant_recipient.default_signer_address();
    let esp_required = fund_amount_esp * U256::from(delegators.len());
    // ETH required: fund each delegator + gas buffer (2x the funding amount)
    let eth_required = fund_amount_eth * U256::from(delegators.len()) * U256::from(2);

    let token_contract = EspToken::new(token_address, &funder_provider);
    let esp_balance = token_contract.balanceOf(funder_address).call().await?;

    tracing::info!(
        "Balance check: have {} ESP, need {} ESP for {} delegators",
        format_ether(esp_balance),
        format_ether(esp_required),
        delegators.len()
    );

    if esp_balance < esp_required {
        return Err(CreateTransactionsError::InsufficientEsp {
            have: format_ether(esp_balance),
            need: format_ether(esp_required),
            delegators: delegators.len(),
        }
        .into());
    }

    let eth_balance = funder_provider.get_balance(funder_address).await?;

    tracing::info!(
        "Balance check: have {} ETH, need {} ETH for {} delegators",
        format_ether(eth_balance),
        format_ether(eth_required),
        delegators.len()
    );

    if eth_balance < eth_required {
        return Err(CreateTransactionsError::InsufficientEth {
            have: format_ether(eth_balance),
            need: format_ether(eth_required),
            recipients: delegators.len(),
        }
        .into());
    }

    // Build tx_inputs for funding phase with backpressure
    let stake_table_address = config.stake_table_address;

    let mut tx_inputs: Vec<TxInput> = Vec::with_capacity(delegators.len() * 3);

    // FundEth for each delegator
    for d in &delegators {
        tx_inputs.push(TxInput {
            phase: TxPhase::FundEth,
            from: funder_address,
            to: d.address(),
            amount: fund_amount_eth,
            delegator_index: None,
        });
    }

    // FundEsp for each delegator
    for d in &delegators {
        tx_inputs.push(TxInput {
            phase: TxPhase::FundEsp,
            from: funder_address,
            to: d.address(),
            amount: fund_amount_esp,
            delegator_index: None,
        });
    }

    // Approve for each delegator
    for d in &delegators {
        tx_inputs.push(TxInput {
            phase: TxPhase::Approve,
            from: d.address(),
            to: stake_table_address,
            amount: fund_amount_esp,
            delegator_index: None,
        });
    }

    // Build wallets map
    let mut wallets: HashMap<Address, EthereumWallet> = HashMap::new();
    wallets.insert(funder_address, grant_recipient.wallet().clone());
    for d in &delegators {
        wallets.insert(d.address(), EthereumWallet::from(d.clone()));
    }

    tracing::info!(
        "funding {} delegators ({} transactions)",
        num_delegators,
        tx_inputs.len()
    );

    let signed_txs = sign_all_transactions(
        &funder_provider,
        &wallets,
        tx_inputs,
        concurrency,
        |input| match input.phase {
            TxPhase::FundEth => TransactionRequest::default()
                .with_to(input.to)
                .with_value(input.amount),
            TxPhase::FundEsp => {
                let call = EspToken::transferCall {
                    to: input.to,
                    value: input.amount,
                };
                TransactionRequest::default()
                    .with_to(token_address)
                    .with_call(&call)
            },
            TxPhase::Approve => {
                let call = EspToken::approveCall {
                    spender: stake_table_address,
                    value: input.amount,
                };
                TransactionRequest::default()
                    .with_to(token_address)
                    .with_call(&call)
            },
            TxPhase::Delegate | TxPhase::Undelegate => unreachable!(),
        },
    )
    .await?;

    let log = TxLog::new(signed_txs);
    execute_signed_tx_log(funder_provider, &log, concurrency, false).await?;

    let query_provider = ProviderBuilder::new().connect_client(shared_client.clone());
    let stake_table = StakeTableV2::new(config.stake_table_address, &query_provider);

    let mut rng = ChaCha20Rng::seed_from_u64(DELEGATOR_SEED);

    tracing::info!("starting continuous churn loop");
    loop {
        let delegator_index = delegator_start_index + rng.gen_range(0..num_delegators);
        let delegator_signer = generate_delegator_signer(delegator_index);
        let delegator_address = delegator_signer.address();

        let mut has_delegation = false;
        for validator in &validator_addresses {
            let delegation_amount = stake_table
                .delegations(*validator, delegator_address)
                .call()
                .await?;
            if !delegation_amount.is_zero() {
                has_delegation = true;
                tracing::info!(
                    "churn: undelegating delegator {} (index {}) from validator {}: {} ESP",
                    delegator_address,
                    delegator_index,
                    validator,
                    format_ether(delegation_amount)
                );

                let delegator_provider = ProviderBuilder::new()
                    .wallet(EthereumWallet::from(delegator_signer.clone()))
                    .connect_client(shared_client.clone());

                Transaction::Undelegate {
                    stake_table: config.stake_table_address,
                    validator: *validator,
                    amount: delegation_amount,
                }
                .send(&delegator_provider)
                .await?
                .assert_success()
                .await?;
                break;
            }
        }

        if !has_delegation {
            let validator = validator_addresses[rng.gen_range(0..validator_addresses.len())];
            let delegation_amount = if min_amount == max_amount {
                min_amount
            } else {
                let range = max_amount - min_amount;
                let random_offset = U256::from(rng.gen_range(0..=u128::MAX)) % range;
                min_amount + random_offset
            };

            tracing::info!(
                "churn: delegating delegator {} (index {}) to validator {}: {} ESP",
                delegator_address,
                delegator_index,
                validator,
                format_ether(delegation_amount)
            );

            let delegator_provider = ProviderBuilder::new()
                .wallet(EthereumWallet::from(delegator_signer))
                .connect_client(shared_client.clone());

            Transaction::Delegate {
                stake_table: config.stake_table_address,
                validator,
                amount: delegation_amount,
            }
            .send(&delegator_provider)
            .await?
            .assert_success()
            .await?;
        }

        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod test {
    use alloy::providers::ext::AnvilApi as _;
    use espresso_types::v0_3::Validator;
    use hotshot_types::signature_key::BLSPubKey;
    use pretty_assertions::assert_matches;
    use rand::rngs::StdRng;

    use super::*;
    use crate::{deploy::TestSystem, info::stake_table_info};

    async fn shared_setup(
        config: DelegationConfig,
    ) -> Result<(Validator<BLSPubKey>, Validator<BLSPubKey>)> {
        let system = TestSystem::deploy().await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![
            TestSystem::gen_keys(&mut rng),
            TestSystem::gen_keys(&mut rng),
        ];

        StakingTransactions::create(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            keys,
            None,
            config,
        )
        .await?
        .apply_all()
        .await?;
        let l1_block_number = system.provider.get_block_number().await?;
        let st = stake_table_info(system.rpc_url, system.stake_table, l1_block_number).await?;

        // The stake table should have 2 validators
        assert_eq!(st.len(), 2);
        let val1 = st[0].clone();
        let val2 = st[1].clone();

        // The validators are not the same
        assert_ne!(val1.account, val2.account);

        Ok((val1, val2))
    }

    #[test_log::test(tokio::test)]
    async fn test_stake_for_demo_equal_amounts() -> Result<()> {
        let (val1, val2) = shared_setup(DelegationConfig::EqualAmounts).await?;

        // The total stake of the validator is equal to it's own delegation
        assert_eq!(val1.delegators.get(&val1.account), Some(&val1.stake));
        assert_eq!(val2.delegators.get(&val2.account), Some(&val2.stake));

        // The are no other delegators
        assert_eq!(val1.delegators.len(), 1);
        assert_eq!(val2.delegators.len(), 1);

        // The stake amounts are equal
        assert_eq!(val1.stake, val2.stake);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_stake_for_demo_variable_amounts() -> Result<()> {
        let (val1, val2) = shared_setup(DelegationConfig::VariableAmounts).await?;

        // The total stake of the validator is equal to it's own delegation
        assert_eq!(val1.delegators.get(&val1.account), Some(&val1.stake));
        assert_eq!(val2.delegators.get(&val2.account), Some(&val2.stake));

        // The are no other delegators
        assert_eq!(val1.delegators.len(), 1);
        assert_eq!(val2.delegators.len(), 1);

        // The stake amounts are not equal
        assert_ne!(val1.stake, val2.stake);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_stake_for_demo_multiple_delegators() -> Result<()> {
        let (val1, val2) = shared_setup(DelegationConfig::MultipleDelegators).await?;

        // The total stake of the validator is not equal to it's own delegation
        assert_ne!(val1.delegators.get(&val1.account), Some(&val1.stake));
        assert_ne!(val2.delegators.get(&val2.account), Some(&val2.stake));

        // The are other delegators
        assert!(val1.delegators.len() > 1);
        assert!(val2.delegators.len() > 1);

        // The stake amounts are not equal
        assert_ne!(val1.stake, val2.stake);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_stake_for_demo_no_self_delegation() -> Result<()> {
        let (val1, val2) = shared_setup(DelegationConfig::NoSelfDelegation).await?;

        // The validators have no self delegation
        assert_eq!(val1.delegators.get(&val1.account), None);
        assert_eq!(val2.delegators.get(&val2.account), None);

        // The are other delegators
        assert!(val1.delegators.len() > 1);
        assert!(val2.delegators.len() > 1);

        // The stake amounts are not equal
        assert_ne!(val1.stake, val2.stake);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_configurable_delegators_per_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![
            TestSystem::gen_keys(&mut rng),
            TestSystem::gen_keys(&mut rng),
        ];

        StakingTransactions::create(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            keys,
            Some(10),
            DelegationConfig::MultipleDelegators,
        )
        .await?
        .apply_all()
        .await?;
        let l1_block_number = system.provider.get_block_number().await?;
        let st = stake_table_info(system.rpc_url, system.stake_table, l1_block_number).await?;

        assert_eq!(st.len(), 2);

        let val1 = &st[0];
        let val2 = &st[1];

        // Each validator should have exactly 10 additional delegators plus self-delegation
        assert_eq!(val1.delegators.len(), 11);
        assert_eq!(val2.delegators.len(), 11);

        Ok(())
    }

    enum Failure {
        Esp,
        Eth,
    }

    #[rstest::rstest]
    #[case::esp(Failure::Esp)]
    #[case::eth(Failure::Eth)]
    #[test_log::test(tokio::test)]
    async fn test_insufficient_balance(#[case] case: Failure) -> Result<()> {
        let system = TestSystem::deploy().await?;

        let drain_address = PrivateKeySigner::random().address();

        match case {
            Failure::Esp => {
                let balance = system
                    .balance(system.provider.default_signer_address())
                    .await?;
                system.transfer(drain_address, balance).await?;
            },
            Failure::Eth => {
                let eth_balance = system
                    .provider
                    .get_balance(system.provider.default_signer_address())
                    .await?;
                // keep a bit for estimateGas calls to succeed
                let drain_amount = eth_balance - parse_ether("1").unwrap();
                system.transfer_eth(drain_address, drain_amount).await?;
            },
        }

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![TestSystem::gen_keys(&mut rng)];

        let result = StakingTransactions::create(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            keys,
            None,
            DelegationConfig::EqualAmounts,
        )
        .await;

        let err = result.expect_err("should fail with insufficient balance");
        match case {
            Failure::Esp => assert_matches!(err, CreateTransactionsError::InsufficientEsp { .. }),
            Failure::Eth => assert_matches!(err, CreateTransactionsError::InsufficientEth { .. }),
        };

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_delegation_below_minimum() -> Result<()> {
        let system = TestSystem::deploy().await?;

        // Set min delegate amount higher than the demo amounts (100-500 ESP range)
        let high_min = parse_ether("2000")?;
        system.set_min_delegate_amount(high_min).await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![TestSystem::gen_keys(&mut rng)];

        // create() only prepares transactions, it doesn't send any, so returning
        // an error here guarantees no transactions were broadcast
        let result = StakingTransactions::create(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            keys,
            None,
            DelegationConfig::EqualAmounts,
        )
        .await;

        assert_matches!(
            result,
            Err(CreateTransactionsError::DelegationBelowMinimum { .. })
        );

        Ok(())
    }

    #[rstest::rstest]
    #[case::equal_amounts(DelegationConfig::EqualAmounts)]
    #[case::variable_amounts(DelegationConfig::VariableAmounts)]
    #[case::multiple_delegators(DelegationConfig::MultipleDelegators)]
    #[case::no_self_delegation(DelegationConfig::NoSelfDelegation)]
    #[test_log::test(tokio::test)]
    async fn test_setup_with_slow_blocks(#[case] config: DelegationConfig) -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.provider.anvil_set_auto_mine(false).await?;
        system.provider.anvil_set_interval_mining(1).await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![
            TestSystem::gen_keys(&mut rng),
            TestSystem::gen_keys(&mut rng),
        ];

        StakingTransactions::create(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            keys,
            None,
            config,
        )
        .await?
        .apply_all()
        .await?;
        let l1_block_number = system.provider.get_block_number().await?;
        let st = stake_table_info(system.rpc_url, system.stake_table, l1_block_number).await?;

        assert_eq!(st.len(), 2);
        assert!(st[0].stake > U256::ZERO);
        assert!(st[1].stake > U256::ZERO);

        if let DelegationConfig::NoSelfDelegation = config {
            assert!(!st[0].delegators.contains_key(&st[0].account));
            assert!(!st[1].delegators.contains_key(&st[1].account));
        }

        Ok(())
    }
}
