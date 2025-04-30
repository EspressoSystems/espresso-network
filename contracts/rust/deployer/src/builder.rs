//! builder pattern for

use alloy::{
    primitives::{Address, U256},
    providers::WalletProvider,
};
use anyhow::{Context, Result};
use derive_builder::Builder;
use hotshot_contract_adapter::sol_types::{LightClientStateSol, StakeTableStateSol};

use crate::{Contract, Contracts, HttpProviderWithWallet};

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
#[derive(Builder, Clone)]
#[builder(setter(strip_option))]
pub struct DeployerArgs {
    deployer: HttpProviderWithWallet,
    #[builder(default)]
    token_recipient: Option<Address>,
    #[builder(default)]
    mock_light_client: bool,
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
}

impl DeployerArgs {
    /// deploy target contracts
    pub async fn deploy(&self, contracts: &mut Contracts, target: Contract) -> Result<()> {
        let provider = &self.deployer;
        let admin = <HttpProviderWithWallet as WalletProvider>::default_signer_address(provider);
        match target {
            Contract::FeeContractProxy => {
                let addr = crate::deploy_fee_contract_proxy(provider, contracts, admin).await?;

                if let Some(multisig) = self.multisig {
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            Contract::EspTokenProxy => {
                let token_recipient = self.token_recipient.unwrap_or(admin);
                let addr =
                    crate::deploy_token_proxy(provider, contracts, admin, token_recipient).await?;

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
                let mut blocks_per_epoch = self.blocks_per_epoch.unwrap();
                let epoch_start_block = self.epoch_start_block.unwrap();

                // TEST-ONLY: if this config is not yet set, we use a large default value
                // to avoid contract complaining about invalid zero-valued blocks_per_epoch.
                // This large value will act as if we are always in epoch 1, which won't conflict
                // with the effective purpose of the real `PublicNetworkConfig`.
                if use_mock && blocks_per_epoch == 0 {
                    blocks_per_epoch = u64::MAX;
                }
                tracing::info!(%blocks_per_epoch, "Upgrading LightClientV2 with ");
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
            },
            Contract::StakeTableProxy => {
                let token_addr = contracts
                    .address(Contract::EspTokenProxy)
                    .context("no ESP token proxy address")?;
                let lc_addr = contracts
                    .address(Contract::LightClientProxy)
                    .context("no LightClient proxy address")?;
                let escrow_period = self.exit_escrow_period.unwrap_or(U256::from(300));
                let addr = crate::deploy_stake_table_proxy(
                    provider,
                    contracts,
                    token_addr,
                    lc_addr,
                    escrow_period,
                    admin,
                )
                .await?;

                if let Some(multisig) = self.multisig {
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            _ => {
                panic!("Deploying {} not supported.", target);
            },
        }
        Ok(())
    }

    /// Deploy all contracts
    pub async fn deploy_all(&self, contracts: &mut Contracts) -> Result<()> {
        self.deploy(contracts, Contract::FeeContractProxy).await?;
        self.deploy(contracts, Contract::EspTokenProxy).await?;
        self.deploy(contracts, Contract::LightClientProxy).await?;
        self.deploy(contracts, Contract::LightClientV2).await?;
        self.deploy(contracts, Contract::StakeTableProxy).await?;
        Ok(())
    }
}
