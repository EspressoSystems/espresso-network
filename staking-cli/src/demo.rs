use alloy::{
    network::EthereumWallet,
    primitives::{utils::parse_ether, Address},
    providers::{ProviderBuilder, WalletProvider},
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

pub async fn stake_for_demo(config: &Config) -> Result<()> {
    let mk_provider = async |account_index| -> Result<_> {
        let signer = MnemonicBuilder::<English>::default()
            .phrase(config.mnemonic.as_str())
            .index(account_index)?
            .build()?;
        let wallet = EthereumWallet::from(signer);
        Ok(ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(config.rpc_url.clone()))
    };
    let deployer_account_index = dotenvy::var("ESPRESSO_DEPLOYER_ACCOUNT_INDEX")?.parse()?;
    let deployer = mk_provider(deployer_account_index).await?;
    let token_address: Address =
        dotenvy::var("ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS")?.parse()?;
    let stake_table_address: Address =
        dotenvy::var("ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS")?.parse()?;
    let token = EspTokenInstance::new(token_address, deployer.clone());
    let amount = parse_ether("1")?;
    for val in 0..=4 {
        // 0 to 4 % commission
        let commission = Commission::try_from(100u16 * val)?;
        // use accounts 10 to 14 of the default mnemonics, hopefully not used for anything else
        let validator_provider = mk_provider(10u32 + val as u32).await?;
        let validator_address = validator_provider.default_signer_address();
        let consensus_private_key = parse_bls_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_{val}"
        ))?)?;
        let state_private_key = parse_state_priv_key(&dotenvy::var(format!(
            "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_{val}"
        ))?)?;

        let receipt = token
            .transfer(validator_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let validator_token = EspTokenInstance::new(token_address, validator_provider.clone());
        let receipt = validator_token
            .approve(stake_table_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        tracing::info!("Deploying validator {val} with commission {commission}");
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

        let receipt = delegate(stake_table, validator_address, amount).await?;
        assert!(receipt.status());
    }
    Ok(())
}
