//! builder pattern for

use alloy::{
    primitives::{Address, U256},
    providers::{Provider, WalletProvider},
};
use anyhow::{Context, Result};
use derive_builder::Builder;
use hotshot_contract_adapter::sol_types::{LightClientStateSol, StakeTableStateSol};

use crate::{Contract, Contracts};

/// Convenient handler that builds all the input arguments ready to be deployed.
/// - `deployer`: deployer's wallet provider
/// - `token_recipient`: initial token holder, same as deployer if None.
/// - `mock_light_client`: flag to indicate whether deploying mocked contract
/// - `genesis_lc_state`: Genesis light client state
/// - `genesis_st_state`: Genesis stake table state
/// - `permissioned_prover`: permissioned light client prover address
/// - `blocks_per_epoch`: epoch length in block height
/// - `epoch_start_block`: block height for the first *activated* epoch
/// - `exit_escrow_period`: exit escrow period for stake table (in seconds)
/// - `multisig`: new owner/multisig that owns all the proxy contracts
/// - `initial_token_supply`: initial token supply for the token contract
/// - `token_name`: name of the token
/// - `token_symbol`: symbol of the token
#[derive(Builder, Clone)]
#[builder(setter(strip_option))]
pub struct DeployerArgs<P: Provider + WalletProvider> {
    deployer: P,
    #[builder(default)]
    token_recipient: Option<Address>,
    #[builder(default)]
    mock_light_client: bool,
    #[builder(default)]
    owned_by_multisig: bool,
    #[builder(default)]
    dry_run: bool,
    #[builder(default)]
    rpc_url: String,
    #[builder(default)]
    genesis_lc_state: Option<LightClientStateSol>,
    #[builder(default)]
    genesis_st_state: Option<StakeTableStateSol>,
    #[builder(default)]
    permissioned_prover: Option<Address>,
    #[builder(default)]
    blocks_per_epoch: Option<u64>,
    #[builder(default)]
    epoch_start_block: Option<u64>,
    #[builder(default)]
    exit_escrow_period: Option<U256>,
    #[builder(default)]
    multisig: Option<Address>,
    #[builder(default)]
    initial_token_supply: Option<U256>,
    #[builder(default)]
    token_name: Option<String>,
    #[builder(default)]
    token_symbol: Option<String>,
}

impl<P: Provider + WalletProvider> DeployerArgs<P> {
    /// deploy target contracts
    pub async fn deploy(&self, contracts: &mut Contracts, target: Contract) -> Result<()> {
        let provider = &self.deployer;
        let admin = provider.default_signer_address();
        match target {
            Contract::FeeContractProxy => {
                let addr = crate::deploy_fee_contract_proxy(provider, contracts, admin).await?;

                if let Some(multisig) = self.multisig {
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            Contract::EspTokenProxy => {
                let token_recipient = self.token_recipient.unwrap_or(admin);
                let token_name = self.token_name.clone().unwrap_or("Espresso".to_string());
                let token_symbol = self.token_symbol.clone().unwrap_or("ESP".to_string());
                let initial_supply = self
                    .initial_token_supply
                    .unwrap_or(U256::from(3590000000u64));
                let addr = crate::deploy_token_proxy(
                    provider,
                    contracts,
                    admin,
                    token_recipient,
                    initial_supply,
                    &token_name,
                    &token_symbol,
                )
                .await?;

                if let Some(multisig) = self.multisig {
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            Contract::LightClientProxy => {
                assert!(
                    self.genesis_lc_state.is_some(),
                    "forget to specify genesis_lc_state()"
                );
                assert!(
                    self.genesis_st_state.is_some(),
                    "forget to specify genesis_st_state()"
                );
                crate::deploy_light_client_proxy(
                    provider,
                    contracts,
                    self.mock_light_client,
                    self.genesis_lc_state.clone().unwrap(),
                    self.genesis_st_state.clone().unwrap(),
                    admin,
                    self.permissioned_prover,
                )
                .await?;
                // NOTE: we don't transfer ownership to multisig, we only do so after V2 upgrade
            },
            Contract::LightClientV2 => {
                assert!(
                    self.blocks_per_epoch.is_some(),
                    "forget to specify blocks_per_epoch()"
                );
                assert!(
                    self.epoch_start_block.is_some(),
                    "forget to specify epoch_start_block()"
                );

                let use_mock = self.mock_light_client;
                let dry_run = self.dry_run;
                let owned_by_multisig = self.owned_by_multisig;
                let mut blocks_per_epoch = self.blocks_per_epoch.unwrap();
                let epoch_start_block = self.epoch_start_block.unwrap();
                let rpc_url = self.rpc_url.clone();

                // TEST-ONLY: if this config is not yet set, we use a large default value
                // to avoid contract complaining about invalid zero-valued blocks_per_epoch.
                // This large value will act as if we are always in epoch 1, which won't conflict
                // with the effective purpose of the real `PublicNetworkConfig`.
                if use_mock && blocks_per_epoch == 0 {
                    blocks_per_epoch = u64::MAX;
                }
                tracing::info!(%blocks_per_epoch, ?dry_run, ?owned_by_multisig, "Upgrading LightClientV2 with ");
                if owned_by_multisig {
                    crate::upgrade_light_client_v2_multisig_owner(
                        provider,
                        contracts,
                        crate::LightClientV2UpgradeParams {
                            is_mock: use_mock,
                            blocks_per_epoch,
                            epoch_start_block,
                            rpc_url,
                            dry_run: Some(dry_run),
                        },
                    )
                    .await?;
                } else {
                    crate::upgrade_light_client_v2(
                        provider,
                        contracts,
                        use_mock,
                        blocks_per_epoch,
                        epoch_start_block,
                    )
                    .await?;

                    if let Some(multisig) = self.multisig {
                        let lc_proxy = contracts
                            .address(Contract::LightClientProxy)
                            .expect("fail to get LightClientProxy address");
                        crate::transfer_ownership(
                            provider,
                            Contract::LightClientProxy,
                            lc_proxy,
                            multisig,
                        )
                        .await?;
                    }
                }
            },
            Contract::StakeTableProxy => {
                let token_addr = contracts
                    .address(Contract::EspTokenProxy)
                    .context("no ESP token proxy address")?;
                let lc_addr = contracts
                    .address(Contract::LightClientProxy)
                    .context("no LightClient proxy address")?;
                let escrow_period = self.exit_escrow_period.unwrap_or(U256::from(300));
                crate::deploy_stake_table_proxy(
                    provider,
                    contracts,
                    token_addr,
                    lc_addr,
                    escrow_period,
                    admin,
                )
                .await?;

                // NOTE: we don't transfer ownership to multisig, we only do so after V2 upgrade
            },
            Contract::StakeTableV2 => {
                crate::upgrade_stake_table_v2(provider, contracts).await?;

                if let Some(multisig) = self.multisig {
                    let stake_table_proxy = contracts
                        .address(Contract::StakeTableProxy)
                        .expect("fail to get StakeTableProxy address");
                    crate::transfer_ownership(
                        provider,
                        Contract::StakeTableProxy,
                        stake_table_proxy,
                        multisig,
                    )
                    .await?;
                }
            },
            _ => {
                panic!("Deploying {} not supported.", target);
            },
        }
        Ok(())
    }

    /// Deploy all contracts up to and including stake table v1
    pub async fn deploy_to_stake_table_v1(&self, contracts: &mut Contracts) -> Result<()> {
        self.deploy(contracts, Contract::FeeContractProxy).await?;
        self.deploy(contracts, Contract::EspTokenProxy).await?;
        self.deploy(contracts, Contract::LightClientProxy).await?;
        self.deploy(contracts, Contract::LightClientV2).await?;
        self.deploy(contracts, Contract::StakeTableProxy).await?;
        Ok(())
    }

    /// Deploy all contracts
    pub async fn deploy_all(&self, contracts: &mut Contracts) -> Result<()> {
        self.deploy_to_stake_table_v1(contracts).await?;
        self.deploy(contracts, Contract::StakeTableV2).await?;
        Ok(())
    }
}
