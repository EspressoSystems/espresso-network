use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use clap::Parser;
use espresso_types::parse_duration;
use ethers::types::Address;
use sequencer_utils::{
    logging,
    stake_table::{update_stake_table, PermissionedStakeTableUpdate},
};
use url::Url;

#[derive(Debug, Clone, Parser)]
struct Options {
    /// RPC URL for the L1 provider.
    #[arg(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545"
    )]
    rpc_url: Url,

    /// Request rate when polling L1.
    #[arg(
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts, so the account indicated by ACCOUNT_INDEX must
    /// be funded with with ETH.
    #[arg(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,

    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[arg(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,

    /// Permissioned stake table contract address.
    #[arg(long, env = "ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_ADDRESS")]
    contract_address: Address,

    /// Path to the toml file containing the update information.
    ///
    /// Schema of toml file:
    /// ```toml
    /// stakers_to_remove = [
    ///   {
    ///     stake_table_key = "BLS_VER_KEY~...",
    ///   },
    /// ]
    ///
    /// new_stakers = [
    ///   {
    ///     stake_table_key = "BLS_VER_KEY~...",
    ///     state_ver_key = "SCHNORR_VER_KEY~...",
    ///     da = true,
    ///     stake = 1, # this value is ignored, but needs to be set
    ///   },
    /// ]
    /// ```
    #[arg(
        long,
        env = "ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_UPDATE_TOML_PATH",
        verbatim_doc_comment
    )]
    update_toml_path: PathBuf,

    #[command(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Options::parse();
    opts.logging.init();
    let update = PermissionedStakeTableUpdate::from_toml_file(&opts.update_toml_path)?;

    update_stake_table(
        opts.rpc_url,
        opts.l1_polling_interval,
        opts.mnemonic,
        opts.account_index,
        opts.contract_address,
        update,
    )
    .await?;

    Ok(())
}
