use alloy::{
    network::{Ethereum, EthereumWallet, TransactionBuilder as _},
    primitives::{
        utils::{format_ether, parse_ether},
        Address, U256,
    },
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, TxFiller,
            WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider, WalletProvider,
    },
    rpc::types::TransactionRequest,
    signers::{
        k256::ecdsa::SigningKey,
        local::{coins_bip39::English, LocalSigner, MnemonicBuilder, PrivateKeySigner},
    },
};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert,
    sol_types::EspToken::{self, EspTokenErrors},
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use url::Url;

use crate::{
    delegation::delegate,
    parse::{parse_bls_priv_key, parse_state_priv_key, Commission},
    registration::register_validator,
    Config,
};

type Prov = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
>;

fn make_provider(rpc_url: &Url, signer: LocalSigner<SigningKey>) -> Prov {
    let wallet = EthereumWallet::from(signer);
    ProviderBuilder::new()
        .wallet(wallet)
        .on_http(rpc_url.clone())
}

pub async fn stake_in_contract_for_test(
    rpc_url: Url,
    grant_recipient: PrivateKeySigner,
    stake_table_address: Address,
    token_address: Address,
    validator_keys: Vec<(PrivateKeySigner, BLSKeyPair, StateKeyPair)>,
    multiple_delegators: bool,
) -> Result<()> {
    tracing::info!("staking to stake table contract for demo");

    tracing::info!("stake table address: {}", stake_table_address);

    let token_signer = make_provider(&rpc_url, grant_recipient.clone());

    tracing::info!("ESP token address: {token_address}");
    let token = EspToken::new(token_address, token_signer.clone());
    let token_balance = token.balanceOf(grant_recipient.address()).call().await?._0;
    tracing::info!(
        "token distributor account {} balance: {} ESP",
        token_signer.default_signer_address(),
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

    for (val_index, (signer, bls_key_pair, state_key_pair)) in
        validator_keys.into_iter().enumerate()
    {
        let validator_provider = make_provider(&rpc_url, signer);
        let validator_address = validator_provider.default_signer_address();

        tracing::info!("fund val {val_index} address: {validator_address}, {fund_amount_eth} ETH");
        let tx = TransactionRequest::default()
            .with_to(validator_address)
            .with_value(fund_amount_eth);
        let receipt = token_signer
            .send_transaction(tx)
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let bal = validator_provider.get_balance(validator_address).await?;

        // 1% commission and more
        let commission = Commission::try_from(100u64 + 10u64 * val_index as u64)?;

        // delegate 100 to 500 ESP
        let delegate_amount = parse_ether("100")? * U256::from(val_index % 5 + 1);
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
            state_key_pair.ver_key(),
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

        if multiple_delegators {
            tracing::info!("adding multiple delegators for validator  {val_index} ");

            let num_delegators = rng.gen_range(2..=5);

            add_multiple_delegators(
                &rpc_url,
                validator_address,
                &token_signer,
                &token,
                stake_table_address,
                token_address,
                &mut rng,
                num_delegators,
            )
            .await?;
        }
    }
    tracing::info!("completed staking for demo");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_multiple_delegators<F: TxFiller<Ethereum>, P: Provider<Ethereum>>(
    rpc_url: &Url,
    validator_address: Address,
    token_signer: &FillProvider<F, P, Ethereum>,
    token: &EspToken::EspTokenInstance<(), FillProvider<F, P, Ethereum>>,
    stake_table_address: Address,
    token_address: Address,
    rng: &mut ChaCha20Rng,
    num_delegators: u64,
) -> Result<()> {
    let fund_amount_esp = parse_ether("1000")?;
    let fund_amount_eth = parse_ether("10")?;

    for delegator_index in 0..num_delegators {
        let delegator_wallet: LocalSigner<SigningKey> = SigningKey::random(rng).into();
        let delegator_address = delegator_wallet.address();
        tracing::info!("delegator {delegator_index}: address {delegator_address}");

        let tx = TransactionRequest::default()
            .with_to(delegator_address)
            .with_value(fund_amount_eth);
        let receipt = token_signer
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

        let delegator_provider = make_provider(rpc_url, delegator_wallet.clone());

        let validator_token = EspToken::new(token_address, delegator_provider.clone());
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
pub async fn stake_for_demo(config: &Config, num_validators: u16) -> Result<()> {
    tracing::info!("staking to stake table contract for demo");

    let mk_signer = |account_index| -> Result<PrivateKeySigner> {
        Ok(MnemonicBuilder::<English>::default()
            .phrase(config.signer.mnemonic.as_ref().unwrap())
            .index(account_index)?
            .build()?)
    };

    let grant_recipient = mk_signer(config.signer.account_index.unwrap())?;

    tracing::info!(
        "grant recipient account for token funding: {}",
        grant_recipient.address()
    );

    let token_address = config.token_address;
    tracing::info!("ESP token address: {}", token_address);
    let stake_table_address = config.stake_table_address;
    tracing::info!("stake table address: {}", stake_table_address);

    let mut validator_keys = vec![];
    for val_index in 0..num_validators {
        let signer = mk_signer(20u32 + val_index as u32)?;
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

    stake_in_contract_for_test(
        config.rpc_url.clone(),
        grant_recipient,
        config.stake_table_address,
        config.token_address,
        validator_keys,
        false,
    )
    .await?;

    tracing::info!("completed staking for demo");
    Ok(())
}
