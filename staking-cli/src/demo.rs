use alloy::{
    network::EthereumWallet,
    primitives::{
        utils::{format_ether, parse_ether},
        U256,
    },
    providers::{Provider, ProviderBuilder, WalletProvider},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use anyhow::Result;
use contract_bindings_alloy::{
    esptoken::EspToken::EspTokenInstance, staketable::StakeTable::StakeTableInstance,
};

use crate::{
    delegation::delegate,
    parse::{parse_bls_priv_key, parse_state_priv_key, Commission},
    registration::register_validator,
    Config,
};

/// Register validators, and delegate to themselves for demo purposes.
///
/// The environment variables used only for this function but not for the normal staking CLI are
/// loaded directly from the environment.
///
/// Account indexes 10 to 14 of the dev mnemonic are used for the validator accounts.
pub async fn stake_for_demo(config: &Config) -> Result<()> {
    tracing::info!("staking to stake table contract for demo");

    let mk_provider = async |account_index| -> Result<_> {
        let signer = MnemonicBuilder::<English>::default()
            .phrase(config.mnemonic.clone())
            .index(account_index)?
            .build()?;
        let wallet = EthereumWallet::from(signer);
        Ok(ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(config.rpc_url.clone()))
    };

    let grant_recipient = mk_provider(config.account_index).await?;
    tracing::info!(
        "grant recipient account for token funding: {}",
        grant_recipient.default_signer_address()
    );

    let token_address = config.token_address;
    tracing::info!("ESP token address: {}", token_address);
    let stake_table_address = config.stake_table_address;
    tracing::info!("stake table address: {}", stake_table_address);

    let token = EspTokenInstance::new(token_address, grant_recipient.clone());
    let fund_amount_eth = "1000";
    let fund_amount = parse_ether(fund_amount_eth)?;

    for val_index in 0..=4 {
        // 10 to 14 % commission
        let commission = Commission::try_from(1000u16 + 100u16 * val_index)?;

        // delegate 100 to 500 ESP
        let delegate_amount = parse_ether("100")? * U256::from(val_index + 1);
        let delegate_amount_esp = format_ether(delegate_amount);

        // use accounts 10 to 14 of the default mnemonics, hopefully not used for anything else
        let validator_provider = mk_provider(10u32 + val_index as u32).await?;
        let validator_address = validator_provider.default_signer_address();
        let bal = validator_provider.get_balance(validator_address).await?;
        tracing::info!("validator {val_index} address: {validator_address}, balance: {bal}");
        let consensus_private_key = parse_bls_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_{val_index}"
        ))?)?;
        let state_private_key = parse_state_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_{val_index}"
        ))?)?;

        tracing::info!("transfer {fund_amount_eth} ESP to {validator_address}",);
        let receipt = token
            .transfer(validator_address, fund_amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("approve {fund_amount_eth} ESP for {stake_table_address}",);
        let validator_token = EspTokenInstance::new(token_address, validator_provider.clone());
        let receipt = validator_token
            .approve(stake_table_address, fund_amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("deploy validator {val_index} with commission {commission}");
        let stake_table = StakeTableInstance::new(stake_table_address, validator_provider);
        let receipt = register_validator(
            stake_table.clone(),
            commission,
            validator_address,
            consensus_private_key.into(),
            (&state_private_key).into(),
        )
        .await?;
        assert!(receipt.status());

        tracing::info!(
            "delegate {delegate_amount_esp} ESP for validator {val_index} from {validator_address}"
        );
        let receipt = delegate(stake_table, validator_address, delegate_amount).await?;
        assert!(receipt.status());
    }
    tracing::info!("completed staking for demo");
    Ok(())
}
