use std::fmt;

use alloy::{
    network::{EthereumWallet, TransactionBuilder as _},
    primitives::{
        Address, U256,
        utils::{format_ether, parse_ether},
    },
    providers::{Provider, ProviderBuilder, WalletProvider},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use clap::ValueEnum;
use espresso_contract_deployer::{build_provider, build_random_provider, build_signer};
use hotshot_contract_adapter::{
    evm::DecodeRevert,
    sol_types::EspToken::{self, EspTokenErrors},
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use url::Url;

use crate::{
    Config,
    delegation::delegate,
    parse::{Commission, parse_bls_priv_key, parse_state_priv_key},
    registration::register_validator,
};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum DelegationConfig {
    EqualAmounts,
    #[default]
    VariableAmounts,
    MultipleDelegators,
}

// Manual implementation to match parsing of clap's ValueEnum
impl fmt::Display for DelegationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DelegationConfig::EqualAmounts => "equal-amounts",
            DelegationConfig::VariableAmounts => "variable-amounts",
            DelegationConfig::MultipleDelegators => "multiple-delegators",
        };
        write!(f, "{s}")
    }
}

/// Setup validator by sending them tokens and ethers, and registering them on stake table
pub async fn setup_stake_table_contract_for_test(
    rpc_url: Url,
    token_holder: &(impl Provider + WalletProvider),
    stake_table_address: Address,
    token_address: Address,
    validators: Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)>,
    config: DelegationConfig,
) -> Result<()> {
    tracing::info!(%stake_table_address, "staking to stake table contract for demo");

    let token_holder_addr = token_holder.default_signer_address();

    tracing::info!("ESP token address: {token_address}");
    let token = EspToken::new(token_address, token_holder);
    let token_balance = token.balanceOf(token_holder_addr).call().await?._0;
    tracing::info!(
        "token distributor account {} balance: {} ESP",
        token_holder_addr,
        format_ether(token_balance)
    );
    if token_balance.is_zero() {
        panic!("grant recipient has no ESP tokens, funding won't work");
    }

    let fund_amount_esp = parse_ether("1000")?;
    let fund_amount_eth = parse_ether("10")?;

    // Set up deterministic rng
    let seed = [42u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    for (val_index, (signer, bls_key_pair, state_key_pair)) in validators.into_iter().enumerate() {
        let validator_address = signer.address();
        let validator_wallet: EthereumWallet = EthereumWallet::from(signer);
        let validator_provider = ProviderBuilder::new()
            .wallet(validator_wallet)
            .on_http(rpc_url.clone());

        tracing::info!("fund val {val_index} address: {validator_address}, {fund_amount_eth} ETH");
        let tx = TransactionRequest::default()
            .with_to(validator_address)
            .with_value(fund_amount_eth);
        let receipt = token_holder
            .send_transaction(tx)
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let bal = validator_provider.get_balance(validator_address).await?;

        // 1% commission and more
        let commission = Commission::try_from(100u64 + 10u64 * val_index as u64)?;

        // delegate 100 to 500 ESP
        let delegate_amount = match config {
            DelegationConfig::EqualAmounts => parse_ether("100")?,
            DelegationConfig::MultipleDelegators | DelegationConfig::VariableAmounts => {
                parse_ether("100")? * U256::from(val_index % 5 + 1)
            },
        };
        let delegate_amount_esp = format_ether(delegate_amount);

        tracing::info!("validator {val_index} address: {validator_address}, balance: {bal}");

        tracing::info!("transfer {fund_amount_esp} ESP to {validator_address}",);
        let receipt = token
            .transfer(validator_address, fund_amount_esp)
            .send()
            .await
            .maybe_decode_revert::<EspTokenErrors>()?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("approve {fund_amount_esp} ESP for {stake_table_address}",);
        let validator_token = EspToken::new(token_address, validator_provider.clone());
        let receipt = validator_token
            .approve(stake_table_address, fund_amount_esp)
            .send()
            .await
            .maybe_decode_revert::<EspTokenErrors>()?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("deploy validator {val_index} with commission {commission}");
        let receipt = register_validator(
            &validator_provider,
            stake_table_address,
            commission,
            validator_address,
            bls_key_pair,
            state_key_pair,
        )
        .await?;
        assert!(receipt.status());

        tracing::info!(
            "delegate {delegate_amount_esp} ESP for validator {val_index} from {validator_address}"
        );
        let receipt = delegate(
            &validator_provider,
            stake_table_address,
            validator_address,
            delegate_amount,
        )
        .await?;
        assert!(receipt.status());

        match config {
            DelegationConfig::EqualAmounts | DelegationConfig::VariableAmounts => {
                tracing::debug!("not adding extra delegators");
            },
            DelegationConfig::MultipleDelegators => {
                tracing::info!("adding multiple delegators for validator {val_index} ");
                let num_delegators = rng.gen_range(2..=5);
                add_multiple_delegators(
                    &rpc_url,
                    validator_address,
                    token_holder,
                    stake_table_address,
                    token_address,
                    &mut rng,
                    num_delegators,
                )
                .await?;
            },
        }
    }
    tracing::info!("completed staking for demo");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_multiple_delegators(
    rpc_url: &Url,
    validator_address: Address,
    token_holder: &(impl Provider + WalletProvider),
    stake_table_address: Address,
    token_address: Address,
    rng: &mut ChaCha20Rng,
    num_delegators: u64,
) -> Result<()> {
    let token = EspToken::new(token_address, token_holder);
    let fund_amount_esp = parse_ether("1000")?;
    let fund_amount_eth = parse_ether("10")?;

    for delegator_index in 0..num_delegators {
        let delegator_provider = build_random_provider(rpc_url.clone());
        let delegator_address = delegator_provider.default_signer_address();

        tracing::info!("delegator {delegator_index}: address {delegator_address}");

        let tx = TransactionRequest::default()
            .with_to(delegator_address)
            .with_value(fund_amount_eth);
        let receipt = token_holder
            .send_transaction(tx)
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("delegator {delegator_index}: funded with {fund_amount_eth} ETH");

        let random_amount: u64 = rng.gen_range(100..=500);
        let delegate_amount = parse_ether(&random_amount.to_string())?;
        let delegate_amount_esp = format_ether(delegate_amount);

        let receipt = token
            .transfer(delegator_address, fund_amount_esp)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("delegator {delegator_index}: received {fund_amount_esp} ESP");

        let validator_token = EspToken::new(token_address, &delegator_provider);
        let receipt = validator_token
            .approve(stake_table_address, delegate_amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!(
            "delegator {delegator_index}: approved {delegate_amount_esp} ESP to stake table"
        );

        let receipt = delegate(
            &delegator_provider,
            stake_table_address,
            validator_address,
            delegate_amount,
        )
        .await?;
        assert!(receipt.status());

        tracing::info!("delegator {delegator_index}: delegation complete");
    }

    Ok(())
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

    let token_address = config.token_address;
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

    setup_stake_table_contract_for_test(
        config.rpc_url.clone(),
        &grant_recipient,
        config.stake_table_address,
        config.token_address,
        validator_keys,
        delegation_config,
    )
    .await?;

    tracing::info!("completed staking for demo");
    Ok(())
}

#[cfg(test)]
mod test {
    use espresso_types::v0_3::Validator;
    use hotshot_types::signature_key::BLSPubKey;
    use rand::rngs::StdRng;
    use sequencer_utils::test_utils::setup_test;

    use super::*;
    use crate::{deploy::TestSystem, info::stake_table_info};

    async fn shared_setup(
        config: DelegationConfig,
    ) -> Result<(Validator<BLSPubKey>, Validator<BLSPubKey>)> {
        setup_test();
        let system = TestSystem::deploy().await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let keys = vec![
            TestSystem::gen_keys(&mut rng),
            TestSystem::gen_keys(&mut rng),
        ];

        setup_stake_table_contract_for_test(
            system.rpc_url.clone(),
            &system.provider,
            system.stake_table,
            system.token,
            keys,
            config,
        )
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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
}
