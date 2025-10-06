use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
};

use alloy::{
    contract::Error as ContractError,
    network::{Ethereum, EthereumWallet},
    primitives::{
        utils::{format_ether, parse_ether},
        Address, U256,
    },
    providers::{PendingTransactionBuilder, Provider, ProviderBuilder, WalletProvider},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    transports::TransportError,
};
use anyhow::Result;
use clap::ValueEnum;
use espresso_contract_deployer::{build_provider, build_signer, HttpProviderWithWallet};
use futures_util::future;
use hotshot_contract_adapter::sol_types::EspToken;
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use thiserror::Error;
use url::Url;

use crate::{
    delegation::{approve, delegate},
    funding::{send_esp, send_eth},
    info::fetch_token_address,
    parse::{parse_bls_priv_key, parse_state_priv_key, Commission, ParseCommissionError},
    receipt::ReceiptExt as _,
    registration::register_validator,
    signature::NodeSignatures,
    Config,
};

#[derive(Debug, Error)]
pub enum CreateError {
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
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Commission(#[from] ParseCommissionError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
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

#[derive(Clone, Debug)]
enum StakeTableTx<P> {
    SendEth {
        provider: P,
        to: Address,
        amount: U256,
    },
    SendEsp {
        provider: P,
        token: Address,
        to: Address,
        amount: U256,
    },
    RegisterValidator {
        provider: P,
        stake_table: Address,
        commission: Commission,
        payload: Box<NodeSignatures>,
    },
    Approve {
        provider: P,
        token: Address,
        stake_table: Address,
        amount: U256,
    },
    Delegate {
        provider: P,
        stake_table: Address,
        validator: Address,
        amount: U256,
    },
}

impl<P: Provider + Clone> StakeTableTx<P> {
    async fn send(self) -> Result<PendingTransactionBuilder<Ethereum>> {
        match self {
            Self::SendEth {
                provider,
                to,
                amount,
            } => send_eth(provider, to, amount).await,
            Self::SendEsp {
                provider,
                token,
                to,
                amount,
            } => send_esp(provider, token, to, amount).await,
            Self::RegisterValidator {
                provider,
                stake_table,
                commission,
                payload,
            } => register_validator(provider, stake_table, commission, *payload).await,
            Self::Approve {
                provider,
                token,
                stake_table,
                amount,
            } => approve(provider, token, stake_table, amount).await,
            Self::Delegate {
                provider,
                stake_table,
                validator,
                amount,
            } => delegate(provider, stake_table, validator, amount).await,
        }
    }
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
pub struct StakingTransactions<P> {
    funding: VecDeque<StakeTableTx<P>>,
    registration: VecDeque<StakeTableTx<P>>,
    approvals: VecDeque<StakeTableTx<P>>,
    delegations: VecDeque<StakeTableTx<P>>,
    current_phase: SetupPhase,
}

impl<P: Provider + Clone> StakingTransactions<P> {
    fn current_group_mut(&mut self) -> &mut VecDeque<StakeTableTx<P>> {
        match self.current_phase {
            SetupPhase::Funding => &mut self.funding,
            SetupPhase::Approval => &mut self.approvals,
            SetupPhase::Registration => &mut self.registration,
            SetupPhase::Delegation => &mut self.delegations,
        }
    }

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
    pub async fn apply_all(self) -> Result<Vec<TransactionReceipt>> {
        let mut all_receipts = vec![];

        for mut group in [
            self.funding,
            self.approvals,
            self.registration,
            self.delegations,
        ] {
            let mut pending = vec![];
            while let Some(tx) = group.pop_front() {
                pending.push(tx.send().await?);
            }

            let receipts = future::try_join_all(
                pending
                    .into_iter()
                    .map(|p| async move { p.assert_success().await }),
            )
            .await?;
            all_receipts.extend(receipts);
        }

        tracing::info!("completed all staking transactions");

        Ok(all_receipts)
    }

    fn pop_next(&mut self) -> Option<StakeTableTx<P>> {
        loop {
            if let Some(tx) = self.current_group_mut().pop_front() {
                return Some(tx);
            }
            self.current_phase = self.current_phase.next()?;
        }
    }

    /// Sends and awaits one transaction
    ///
    /// The caller can use this function to rate limit changes to the L1 stake
    /// table contract during setup.
    pub async fn apply_one(&mut self) -> Result<Option<TransactionReceipt>> {
        let Some(tx) = self.pop_next() else {
            return Ok(None);
        };
        let pending = tx.send().await?;
        Ok(Some(pending.assert_success().await?))
    }

    /// Sends and awaits receipts on all funding and approval transactions
    ///
    /// If the caller wants more control but quickly get to a point where actual
    /// changes are made to the stake table it is useful to call this function
    /// first.
    ///
    /// This processes funding and approvals with a synchronization point between them.
    pub async fn apply_prerequisites(&mut self) -> Result<Vec<TransactionReceipt>> {
        if !matches!(self.current_phase, SetupPhase::Funding) {
            return Err(anyhow::anyhow!("apply_prerequisites must be called first"));
        }

        let mut all_receipts = vec![];

        for group in [&mut self.funding, &mut self.approvals] {
            if group.is_empty() {
                continue;
            }

            let mut pending = vec![];
            while let Some(tx) = group.pop_front() {
                pending.push(tx.send().await?);
            }

            let receipts = future::try_join_all(
                pending
                    .into_iter()
                    .map(|p| async move { p.assert_success().await }),
            )
            .await?;
            all_receipts.extend(receipts);
        }

        self.current_phase = SetupPhase::Registration;

        Ok(all_receipts)
    }
}

impl StakingTransactions<HttpProviderWithWallet> {
    /// Create staking transactions for test setup
    ///
    /// Prepares all transactions needed to setup the stake table with validators and delegations.
    /// The transactions can be applied with different levels of concurrency using the methods on
    /// the returned instance.
    ///
    /// Assumptions:
    ///
    /// - Full control of Validators Ethereum wallets and the Ethereum node. Transactions are
    ///   constructed in a way that they should always succeed, but if they don't the easiest fix is
    ///   probably to re-deploy the ethereum network. Recovery, replacing of transactions is not
    ///   implemented.
    ///
    /// - Running against a single node Ethereum network. The nonces are not tracked in our rust
    ///   code and we rely on sending transactions with "pending" block tag. This won't work well
    ///   for a real, distributed ethereum network.
    ///
    /// - Nobody else is using the Ethereum accounts for anything else between calling this function
    ///   and applying the returned transactions.
    ///
    /// Requirements:
    ///
    /// - token_holder: requires Eth to fund validators and delegators, ESP tokens to fund delegators
    ///
    /// Amounts used for funding, delegations, number of delegators are chosen somewhat arbitrarily.
    pub async fn create(
        rpc_url: Url,
        token_holder: &(impl Provider + WalletProvider<Wallet = EthereumWallet>),
        stake_table: Address,
        validators: Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)>,
        config: DelegationConfig,
    ) -> Result<Self, CreateError> {
        tracing::info!(%stake_table, "staking to stake table contract for demo");

        let token = fetch_token_address(rpc_url.clone(), stake_table).await?;

        let token_holder_provider = ProviderBuilder::new()
            .wallet(token_holder.wallet().clone())
            .connect_http(rpc_url.clone());

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
                for _ in 0..rng.gen_range(2..=5) {
                    let random_amount: u64 = rng.gen_range(100..=500);
                    delegator_info.push(DelegatorConfig {
                        validator: validator.signer.address(),
                        signer: PrivateKeySigner::random(),
                        delegate_amount: parse_ether(&random_amount.to_string()).unwrap(),
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
                provider: token_holder_provider.clone(),
                to: address,
                amount: fund_amount_eth,
            });
        }

        for delegator in &delegator_info {
            funding.push_back(StakeTableTx::SendEsp {
                provider: token_holder_provider.clone(),
                token,
                to: delegator.signer.address(),
                amount: fund_amount_esp,
            });
        }

        // Only create one provider per address to avoid nonce errors.
        let mut providers: HashMap<Address, _> = HashMap::new();

        let mut registration = VecDeque::new();

        for validator in &validator_info {
            let address = validator.signer.address();
            let provider = providers.entry(address).or_insert_with(|| {
                ProviderBuilder::new()
                    .wallet(EthereumWallet::from(validator.signer.clone()))
                    .connect_http(rpc_url.clone())
            });

            let payload =
                NodeSignatures::create(address, &validator.bls_key_pair, &validator.state_key_pair);
            registration.push_back(StakeTableTx::RegisterValidator {
                provider: provider.clone(),
                stake_table,
                commission: validator.commission,
                payload: Box::new(payload),
            });
        }

        let mut approvals = VecDeque::new();
        let mut delegations = VecDeque::new();

        for delegator in &delegator_info {
            let address = delegator.signer.address();
            let provider = providers.entry(address).or_insert_with(|| {
                ProviderBuilder::new()
                    .wallet(EthereumWallet::from(delegator.signer.clone()))
                    .connect_http(rpc_url.clone())
            });

            approvals.push_back(StakeTableTx::Approve {
                provider: provider.clone(),
                token,
                stake_table,
                amount: delegator.delegate_amount,
            });

            delegations.push_back(StakeTableTx::Delegate {
                provider: provider.clone(),
                stake_table,
                validator: delegator.validator,
                amount: delegator.delegate_amount,
            });
        }

        let total_esp_required = fund_amount_esp * U256::from(delegator_info.len());
        let total_eth_required = fund_amount_eth * U256::from(eth_recipients.len()) * U256::from(2);

        if token_balance < total_esp_required {
            return Err(CreateError::InsufficientEsp {
                have: format_ether(token_balance),
                need: format_ether(total_esp_required),
                delegators: delegator_info.len(),
            });
        }

        let eth_balance = token_holder_provider.get_balance(token_holder_addr).await?;
        if eth_balance < total_eth_required {
            return Err(CreateError::InsufficientEth {
                have: format_ether(eth_balance),
                need: format_ether(total_eth_required),
                recipients: eth_recipients.len(),
            });
        }

        tracing::info!(
            "balance check passed: {} ESP available (need {}), {} ETH available (need {} \
             including gas buffer)",
            format_ether(token_balance),
            format_ether(total_esp_required),
            format_ether(eth_balance),
            format_ether(total_eth_required)
        );

        Ok(StakingTransactions {
            funding,
            registration,
            approvals,
            delegations,
            current_phase: SetupPhase::Funding,
        })
    }
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
    delegation_config: DelegationConfig,
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
            20u32 + val_index as u32,
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
        delegation_config,
    )
    .await?
    .apply_all()
    .await?;

    Ok(())
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

    enum FailureCase {
        Esp,
        Eth,
    }

    #[rstest::rstest]
    #[case::esp(FailureCase::Esp)]
    #[case::eth(FailureCase::Eth)]
    #[test_log::test(tokio::test)]
    async fn test_insufficient_balance(#[case] case: FailureCase) -> Result<()> {
        let system = TestSystem::deploy().await?;

        let drain_address = PrivateKeySigner::random().address();

        match case {
            FailureCase::Esp => {
                let balance = system
                    .balance(system.provider.default_signer_address())
                    .await?;
                system.transfer(drain_address, balance).await?;
            },
            FailureCase::Eth => {
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
            DelegationConfig::EqualAmounts,
        )
        .await;

        let err = result.expect_err("should fail with insufficient balance");
        match case {
            FailureCase::Esp => assert_matches!(err, CreateError::InsufficientEsp { .. }),
            FailureCase::Eth => assert_matches!(err, CreateError::InsufficientEth { .. }),
        }

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
