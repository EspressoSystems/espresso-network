use std::{fs::File, io::stdout, path::PathBuf, thread::sleep, time::Duration};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use anyhow::Context;
use clap::Parser;
use espresso_types::{config::PublicNetworkConfig, parse_duration, SeqTypes};
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use hotshot_state_prover::service::light_client_genesis;
use sequencer_utils::{
    deployer::{self, transfer_ownership, Contract, Contracts, DeployedContracts},
    logging,
    stake_table::PermissionedStakeTableConfig,
};
use tide_disco::error::ServerError;
use url::Url;
use vbs::version::StaticVersion;

/// Deploy contracts needed to run the sequencer.
///
/// This script deploys contracts needed to run the sequencer to an L1. It outputs a .env file
/// containing the addresses of the deployed contracts.
///
/// This script can also be used to do incremental deployments. The only contract addresses
/// needed to configure the sequencer network are ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS,
/// ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS and (soon) PERMISSIONED_STAKE_TABLE_ADDRESS.
/// These contracts, however, have dependencies, and a full deployment involves several
/// contracts. Some of these contracts, especially libraries may already have been deployed, or
/// perhaps one of the top-level contracts has been deployed and we only need to deploy the other
/// one.
///
/// It is possible to pass in the addresses of already deployed contracts, in which case those
/// addresses will be used in place of deploying a new contract wherever that contract is required
/// in the deployment process. The generated .env file will include all the addresses passed in as
/// well as those newly deployed.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// A JSON-RPC endpoint for the L1 to deploy to.
    #[clap(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545"
    )]
    rpc_url: Url,

    /// Request rate when polling L1.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// URL of a sequencer node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts, so the account indicated by ACCOUNT_INDEX must
    /// be funded with with ETH.
    #[clap(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,

    /// Address for the multisig wallet that will be the admin
    ///
    /// If provided, this the multisig wallet that will be able to upgrade contracts and execute
    /// admin only functions on contracts. If not provided, admin power for all contracts will be
    /// held by the account used to deploy the contracts (determined from MNEMONIC, ACCOUNT_INDEX).
    #[clap(
        long,
        name = "MULTISIG_ADDRESS",
        env = "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS"
    )]
    multisig_address: Option<Address>,

    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[clap(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,

    // /// Only deploy the given groups of related contracts.
    // #[clap(long, value_delimiter = ',')]
    // only: Option<Vec<ContractGroup>>,
    /// Option to deploy fee contracts
    #[clap(long, default_value = "false")]
    deploy_fee: bool,
    /// Option to deploy permissioned stake table contracts
    #[clap(long, default_value = "false")]
    deploy_permissioned_stake_table: bool,
    /// Option to deploy LightClient V1 and proxy
    #[clap(long, default_value = "false")]
    deploy_light_client_v1: bool,
    /// Option to upgrade to LightClient V2
    #[clap(long, default_value = "false")]
    upgrade_light_client_v2: bool,
    #[clap(long, default_value = "false")]
    deploy_esp_token: bool,
    #[clap(long, default_value = "false")]
    deploy_stake_table: bool,

    /// Write deployment results to OUT as a .env file.
    ///
    /// If not provided, the results will be written to stdout.
    #[clap(short, long, name = "OUT", env = "ESPRESSO_DEPLOYER_OUT_PATH")]
    out: Option<PathBuf>,

    #[clap(flatten)]
    contracts: DeployedContracts,

    /// If toggled, launch a mock LightClient contract with a smaller verification key for testing.
    /// Applies to both V1 and V2 of LightClient.
    #[clap(short, long)]
    pub use_mock: bool,

    /// Stake table capacity for the prover circuit
    #[clap(short, long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY", default_value_t = STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,

    /// Permissioned prover address for light client contract.
    ///
    /// If the light client contract is being deployed and this is set, the prover will be
    /// permissioned so that only this address can update the light client state. Otherwise, proving
    /// will be permissionless.
    ///
    /// If the light client contract is not being deployed, this option is ignored.
    #[clap(long, env = "ESPRESSO_SEQUENCER_PERMISSIONED_PROVER")]
    permissioned_prover: Option<Address>,

    /// A toml file with the initial stake table.
    ///
    /// Schema:
    ///
    /// public_keys = [
    ///   {
    ///     stake_table_key = "BLS_VER_KEY~...",
    ///     state_ver_key = "SCHNORR_VER_KEY~...",
    ///     da = true,
    ///     stake = 1, # this value is ignored, but needs to be set
    ///   },
    /// ]
    #[clap(long, env = "ESPRESSO_SEQUENCER_INITIAL_PERMISSIONED_STAKE_TABLE_PATH")]
    initial_stake_table_path: Option<PathBuf>,

    /// Exit escrow period for the stake table contract.
    ///
    /// This is the period for which stake table contract will retain funds after withdrawals have
    /// been requested. It should be set to a value that is at least 3 hotshot epochs plus ample
    /// time to allow for submission of slashing evidence. Initially it will probably be around one
    /// week.
    #[clap(long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_EXIT_ESCROW_PERIOD", value_parser = parse_duration)]
    exit_escrow_period: Option<Duration>,

    /// The address that the tokens will be minted to.
    ///
    /// If unset the tokens will be minted to the deployer account.
    #[clap(long, env = "ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS")]
    initial_token_grant_recipient: Option<Address>,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    let mut contracts = Contracts::from(opt.contracts);

    let signer = MnemonicBuilder::<English>::default()
        .phrase(opt.mnemonic)
        .index(opt.account_index)
        .expect("wrong mnemonic or index")
        .build()
        .expect("fail to build signer");
    let deployer = signer.address();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new().wallet(wallet).on_http(opt.rpc_url);

    if opt.deploy_fee {
        let owner = match opt.multisig_address {
            Some(multisig) => multisig,
            None => deployer,
        };
        let _fee_proxy_addr =
            deployer::deploy_fee_contract_proxy(&provider, &mut contracts, owner).await?;
    }

    if opt.deploy_permissioned_stake_table {
        let initial_stake_table = if let Some(path) = opt.initial_stake_table_path {
            tracing::info!("Loading initial stake table from {:?}", path);
            PermissionedStakeTableConfig::<SeqTypes>::from_toml_file(&path)?.into()
        } else {
            vec![]
        };

        let stake_table_addr = deployer::deploy_permissioned_stake_table(
            &provider,
            &mut contracts,
            initial_stake_table,
        )
        .await?;
        if let Some(multisig) = opt.multisig_address {
            transfer_ownership(
                &provider,
                Contract::PermissonedStakeTable,
                stake_table_addr,
                multisig,
            )
            .await?;
        }
    }

    if opt.deploy_light_client_v1 {
        let (genesis_state, genesis_stake) =
            light_client_genesis(&opt.sequencer_url, opt.stake_table_capacity).await?;
        let lc_proxy_addr = deployer::deploy_light_client_proxy(
            &provider,
            &mut contracts,
            opt.use_mock,
            genesis_state,
            genesis_stake,
            deployer,
            opt.permissioned_prover,
        )
        .await?;
        // NOTE: in actual production, we should transfer ownership to multisig at this point,
        // and only upgrade from multisig, but here for tests and demo, we only transfer ownership
        // after upgrade so that the deployer can still upgrade.

        if opt.upgrade_light_client_v2 {
            // fetch epoch length from HotShot config
            let config_url = opt.sequencer_url.join("/config/hotshot")?;
            // Request the configuration until it is successful
            let (mut blocks_per_epoch, epoch_start_block) = loop {
                match surf_disco::Client::<ServerError, StaticVersion<0, 1>>::new(
                    config_url.clone(),
                )
                .get::<PublicNetworkConfig>(config_url.as_str())
                .send()
                .await
                {
                    Ok(resp) => {
                        let config = resp.hotshot_config();
                        break (config.blocks_per_epoch(), config.epoch_start_block());
                    },
                    Err(e) => {
                        tracing::error!("Failed to fetch the network config: {e}");
                        sleep(Duration::from_secs(5));
                    },
                }
            };

            // TEST-ONLY: if this config is not yet set, we use a large default value
            // to avoid contract complaining about invalid zero-valued blocks_per_epoch.
            // This large value will act as if we are always in epoch 1, which won't conflict
            // with the effective purpose of the real `PublicNetworkConfig`.
            if opt.use_mock && blocks_per_epoch == 0 {
                blocks_per_epoch = u64::MAX;
            }
            tracing::info!(%blocks_per_epoch, "Upgrading LightClientV2 with ");

            deployer::upgrade_light_client_v2(
                &provider,
                &mut contracts,
                opt.use_mock,
                blocks_per_epoch,
                epoch_start_block,
            )
            .await?;
        }

        // NOTE: see the comment during LC V1 deployment, we defer ownership transfer to multisig here.
        if let Some(multisig) = opt.multisig_address {
            transfer_ownership(
                &provider,
                Contract::LightClientProxy,
                lc_proxy_addr,
                multisig,
            )
            .await?;
        }
    }

    if opt.deploy_esp_token {
        let recipient = match opt.initial_token_grant_recipient {
            Some(r) => r,
            None => deployer,
        };
        let token_proxy_addr =
            deployer::deploy_token_proxy(&provider, &mut contracts, deployer, recipient).await?;

        if let Some(multisig) = opt.multisig_address {
            transfer_ownership(
                &provider,
                Contract::EspTokenProxy,
                token_proxy_addr,
                multisig,
            )
            .await?;
        }
    }

    if opt.deploy_stake_table {
        let token_addr = contracts
            .address(Contract::EspTokenProxy)
            .context("no ESP token proxy address")?;
        let lc_addr = contracts
            .address(Contract::LightClientProxy)
            .context("no LightClient proxy address")?;
        let escrow_period = U256::from(
            opt.exit_escrow_period
                .context("no exit escrow period")?
                .as_secs(),
        );
        let stake_table_proxy_addr = deployer::deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            escrow_period,
            deployer,
        )
        .await?;

        if let Some(multisig) = opt.multisig_address {
            transfer_ownership(
                &provider,
                Contract::StakeTableProxy,
                stake_table_proxy_addr,
                multisig,
            )
            .await?;
        }
    }

    if let Some(out) = &opt.out {
        let file = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(out)?;
        contracts.write(file)?;
    } else {
        contracts.write(stdout())?;
    }

    Ok(())
}
